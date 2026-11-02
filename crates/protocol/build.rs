//! Records which commit this is built from, so a daemon can tell how far
//! behind upstream it is and frontends can show what runs where.

use std::path::Path;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let output = Command::new("git")
        .arg("-C")
        .arg(manifest)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn main() {
    let commit = git(&["rev-parse", "--short=10", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());
    println!("cargo::rustc-env=PLUSYSMON_COMMIT={commit}");
    // Rebuild when HEAD moves: a commit, checkout or pull writes these.
    if let Some(dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        for file in ["HEAD", "logs/HEAD"] {
            let path = Path::new(&dir).join(file);
            if path.exists() {
                println!("cargo::rerun-if-changed={}", path.display());
            }
        }
    }
    println!("cargo::rerun-if-changed=build.rs");
}
