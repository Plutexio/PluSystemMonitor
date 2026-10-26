use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use cxx_qt_build::{CxxQtBuilder, QResource, QResources, QmlFile, QmlModule};

/// QML singletons, by path; every other `.qml` file under qml/ is a type.
const SINGLETONS: &[&str] = &[
    "qml/design/Appearance.qml",
    "qml/Widgets.qml",
    "qml/Format.qml",
    "qml/Sources.qml",
];

fn files_under(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files_under(&path, extension, out);
        } else if path.extension().is_some_and(|e| e == extension) {
            out.push(path);
        }
    }
}

/// Qt's shader baker, from where qmake says Qt keeps its tools.
fn qsb() -> PathBuf {
    if let Some(path) = env::var_os("QSB") {
        return path.into();
    }
    let qmake = env::var("QMAKE").unwrap_or_else(|_| "qmake6".to_owned());
    for query in ["QT_HOST_BINS", "QT_INSTALL_BINS", "QT_HOST_LIBEXECS"] {
        if let Ok(output) = Command::new(&qmake).args(["-query", query]).output() {
            let dir = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            let candidate = Path::new(&dir).join("qsb");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    "qsb".into()
}

/// Bakes every shader in qml/shaders into OUT_DIR, served as
/// qrc:/shaders/<name>.qsb.
fn shaders() -> QResource {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR")).join("shaders");
    fs::create_dir_all(&out).expect("creating the shader folder");
    let mut sources = Vec::new();
    files_under(Path::new("qml/shaders"), "frag", &mut sources);
    let qsb = qsb();
    let mut resource = QResource::new().prefix("/shaders");
    for source in sources {
        let name = format!("{}.qsb", source.file_name().unwrap().to_string_lossy());
        let baked = out.join(&name);
        let status = Command::new(&qsb)
            .arg("--qt6")
            .arg("-o")
            .arg(&baked)
            .arg(&source)
            .status()
            .unwrap_or_else(|e| panic!("running {}: {e}", qsb.display()));
        assert!(status.success(), "{} failed on {}", qsb.display(), source.display());
        println!("cargo::rerun-if-changed={}", source.display());
        resource = resource.file(qt_build_utils::QResourceFile::new(&baked).alias(name));
    }
    resource
}

fn main() {
    let mut qml = Vec::new();
    files_under(Path::new("qml"), "qml", &mut qml);
    qml.retain(|path| !path.starts_with("qml/third_party"));
    let mut module = QmlModule::new("PluSystemMonitor");
    for path in &qml {
        let name = path.to_string_lossy().into_owned();
        let file = QmlFile::from(name.as_str());
        module = module.qml_file(if SINGLETONS.contains(&name.as_str()) {
            file.singleton(true)
        } else {
            file
        });
    }
    println!("cargo::rerun-if-changed=qml");

    // JavaScript libraries are imported by relative path, so they sit beside
    // the QML files in the module's resource folder.
    let mut scripts = Vec::new();
    files_under(Path::new("qml"), "js", &mut scripts);

    CxxQtBuilder::new_qml_module(module)
        .qrc_resources(QResources::new().resource(QResource::new().files(scripts)).resource(shaders()))
        .qt_module("Quick")
        .files(["src/monitor.rs", "src/config.rs"])
        .build();
}
