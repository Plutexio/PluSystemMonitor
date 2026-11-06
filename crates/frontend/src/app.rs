//! Which of the two apps this run is: the system monitor (`plusysmon`) or
//! fan control (`plusysfan`, the same program under another name). They share
//! everything but their window and their preferences file.

use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::Result;

use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum App {
    Monitor,
    Fans,
}

static CURRENT: OnceLock<App> = OnceLock::new();

impl App {
    /// `plusysfan` (the installed symlink) or `--fans` make the fan app.
    pub fn from_args(args: &[String]) -> Self {
        let called = args.first().and_then(|arg0| {
            std::path::Path::new(arg0)
                .file_name()?
                .to_str()
                .map(str::to_owned)
        });
        if called.as_deref() == Some("plusysfan") || args.iter().skip(1).any(|a| a == "--fans") {
            App::Fans
        } else {
            App::Monitor
        }
    }

    pub fn set_current(self) {
        let _ = CURRENT.set(self);
    }

    pub fn current() -> Self {
        *CURRENT.get().unwrap_or(&App::Monitor)
    }

    pub fn name(self) -> &'static str {
        match self {
            App::Monitor => "PluSystemMonitor",
            App::Fans => "PluFan",
        }
    }

    /// The installed `.desktop` file, so Wayland compositors show its icon.
    pub fn desktop_file(self) -> &'static str {
        match self {
            App::Monitor => "plusysmon",
            App::Fans => "plusysfan",
        }
    }

    pub fn root_qml(self) -> &'static str {
        match self {
            App::Monitor => "qrc:/qt/qml/PluSystemMonitor/qml/Main.qml",
            App::Fans => "qrc:/qt/qml/PluSystemMonitor/qml/fans/FansMain.qml",
        }
    }

    pub fn prefs_file(self) -> Result<PathBuf> {
        match self {
            App::Monitor => paths::prefs_file(),
            App::Fans => paths::fan_app_prefs_file(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn picks_the_app_by_name_or_flag() {
        assert_eq!(App::from_args(&args(&["/usr/bin/plusysmon"])), App::Monitor);
        assert_eq!(
            App::from_args(&args(&["/home/me/.local/bin/plusysfan"])),
            App::Fans
        );
        assert_eq!(App::from_args(&args(&["plusysmon", "--fans"])), App::Fans);
        assert_eq!(App::from_args(&args(&[])), App::Monitor);
    }
}
