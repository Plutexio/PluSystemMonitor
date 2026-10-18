//! The `Config` QML singleton: preferences, themes and layout on disk.
//!
//! Structured data crosses into QML as JSON strings; parsing and validation
//! stay on the Rust side so QML only ever sees well-formed values.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Config = super::ConfigRust;

        /// `{ "theme": id, "connection": { "address", "token" } }`
        #[qinvokable]
        fn preferences(&self) -> QString;

        #[qinvokable]
        fn save_preferences(self: Pin<&mut Self>, json: &QString);

        /// Summaries of every theme, for the picker.
        #[qinvokable]
        fn themes(&self) -> QString;

        /// The resolved theme as flat `Theme.qml` properties, or an empty string
        /// (plus `errorOccurred`) if it cannot be loaded. Starts watching its files.
        #[qinvokable]
        fn load_theme(self: Pin<&mut Self>, id: &QString) -> QString;

        /// `file://` URL of the user themes folder, created if missing.
        #[qinvokable]
        fn themes_folder(self: Pin<&mut Self>) -> QString;

        #[qinvokable]
        fn layout(self: Pin<&mut Self>) -> QString;

        #[qinvokable]
        fn default_layout(&self) -> QString;

        #[qinvokable]
        fn save_layout(self: Pin<&mut Self>, json: &QString);

        #[qsignal]
        fn error_occurred(self: Pin<&mut Self>, message: &QString);

        /// A file of the current theme changed on disk.
        #[qsignal]
        fn theme_file_changed(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for Config {}
}

use std::fs;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::layout::{self, Layout};
use crate::paths;
use crate::prefs::Prefs;
use crate::theme;

const WATCH_INTERVAL: Duration = Duration::from_millis(700);

type Watched = Arc<Mutex<Vec<(PathBuf, Option<SystemTime>)>>>;

#[derive(Default)]
pub struct ConfigRust {
    watched: Watched,
    watcher_started: bool,
}

fn modified(path: &PathBuf) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl qobject::Config {
    pub fn preferences(&self) -> QString {
        let prefs = paths::prefs_file()
            .ok()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|text| toml::from_str::<Prefs>(&text).ok())
            .unwrap_or_default();
        QString::from(&serde_json::to_string(&prefs).expect("prefs serialize"))
    }

    pub fn save_preferences(self: Pin<&mut Self>, json: &QString) {
        let result = (|| -> Result<()> {
            let prefs: Prefs =
                serde_json::from_str(&json.to_string()).context("malformed preferences")?;
            paths::write_atomic(&paths::prefs_file()?, &toml::to_string_pretty(&prefs)?)
        })();
        self.report(result);
    }

    pub fn themes(&self) -> QString {
        let dir = paths::themes_dir().ok();
        let list = theme::list(dir.as_deref());
        QString::from(&serde_json::to_string(&list).expect("theme list serializes"))
    }

    pub fn load_theme(mut self: Pin<&mut Self>, id: &QString) -> QString {
        let dir = paths::themes_dir().ok();
        match theme::resolve(&id.to_string(), dir.as_deref()) {
            Ok(resolved) => {
                *self.rust().watched.lock().unwrap() = resolved
                    .files
                    .into_iter()
                    .map(|path| {
                        let seen = modified(&path);
                        (path, seen)
                    })
                    .collect();
                self.as_mut().start_watcher();
                QString::from(&theme::to_qml(&resolved.theme).to_string())
            }
            Err(e) => {
                self.report(Err(e));
                QString::default()
            }
        }
    }

    pub fn themes_folder(self: Pin<&mut Self>) -> QString {
        let result = paths::themes_dir().and_then(|dir| {
            fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
            Ok(dir)
        });
        match result {
            Ok(dir) => QString::from(&format!("file://{}", dir.display())),
            Err(e) => {
                self.report(Err(e));
                QString::default()
            }
        }
    }

    pub fn layout(self: Pin<&mut Self>) -> QString {
        let loaded = paths::layout_file().map(|path| layout::load(&path));
        let layout = match loaded {
            Ok(layout::Loaded::Ok(layout)) => layout,
            Ok(layout::Loaded::Missing) => Layout::builtin_default(),
            Ok(layout::Loaded::Broken(reason)) => {
                let message = format!(
                    "Layout could not be read and was saved as layout.toml.broken: {reason}"
                );
                self.error_occurred(&QString::from(&message));
                Layout::builtin_default()
            }
            Err(e) => {
                self.report(Err(e));
                Layout::builtin_default()
            }
        };
        QString::from(&layout.to_json())
    }

    pub fn default_layout(&self) -> QString {
        QString::from(&Layout::builtin_default().to_json())
    }

    pub fn save_layout(self: Pin<&mut Self>, json: &QString) {
        let result = Layout::from_json(&json.to_string())
            .and_then(|layout| paths::write_atomic(&paths::layout_file()?, &layout.to_toml()));
        self.report(result);
    }

    fn report(self: Pin<&mut Self>, result: Result<()>) {
        if let Err(e) = result {
            self.error_occurred(&QString::from(&format!("{e:#}")));
        }
    }

    /// Polls the current theme's files; cheap enough and needs no inotify plumbing.
    fn start_watcher(mut self: Pin<&mut Self>) {
        if self.rust().watcher_started {
            return;
        }
        self.as_mut().rust_mut().watcher_started = true;
        let watched = self.rust().watched.clone();
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(WATCH_INTERVAL);
                let changed =
                    watched
                        .lock()
                        .unwrap()
                        .iter_mut()
                        .fold(false, |changed, (path, seen)| {
                            let now = modified(path);
                            let differs = now != *seen;
                            *seen = now;
                            changed | differs
                        });
                if changed
                    && qt_thread
                        .queue(|config| config.theme_file_changed())
                        .is_err()
                {
                    break;
                }
            }
        });
    }
}
