//! The daemon's TLS identity: a self-signed certificate made on first run and
//! kept from then on, since clients pin its fingerprint.

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use plusysmon_protocol::cert_fingerprint;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use tokio_rustls::rustls::version::TLS13;
use tokio_rustls::rustls::{ServerConfig, crypto};

pub struct Identity {
    pub acceptor: TlsAcceptor,
    pub fingerprint: String,
}

/// Reads the certificate, generating one when it or its key is missing. A file
/// that exists but can't be used is an error: replacing it would make every
/// client warn that this machine's identity changed.
pub fn load_or_create(cert_path: &Path) -> Result<Identity> {
    let key_path = cert_path.with_file_name("tls-key.der");
    if !(cert_path.exists() && key_path.exists()) {
        generate(cert_path, &key_path)?;
    }
    let cert = fs::read(cert_path).with_context(|| format!("reading {}", cert_path.display()))?;
    let key = fs::read(&key_path).with_context(|| format!("reading {}", key_path.display()))?;
    let fingerprint = cert_fingerprint(&cert);

    let config = ServerConfig::builder_with_provider(Arc::new(crypto::ring::default_provider()))
        .with_protocol_versions(&[&TLS13])?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(cert)],
            PrivatePkcs8KeyDer::from(key).into(),
        )
        .with_context(|| {
            format!(
                "unusable TLS identity; delete {} and {} to make a new one",
                cert_path.display(),
                key_path.display()
            )
        })?;
    Ok(Identity {
        acceptor: TlsAcceptor::from(Arc::new(config)),
        fingerprint,
    })
}

fn generate(cert_path: &Path, key_path: &Path) -> Result<()> {
    if let Some(dir) = cert_path.parent() {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .with_context(|| format!("creating {}", dir.display()))?;
    }
    // Nothing checks the name (clients go by the fingerprint), and rcgen's
    // default validity runs until the year 4096, so it never needs renewing.
    let generated = rcgen::generate_simple_self_signed(["plusysmond".to_owned()])
        .context("generating a TLS certificate")?;

    // The certificate goes last and by rename: if we die in between, its absence
    // makes the next start generate both again.
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(key_path)
        .and_then(|mut f| f.write_all(&generated.signing_key.serialize_der()))
        .with_context(|| format!("writing {}", key_path.display()))?;
    let tmp = cert_path.with_extension("tmp");
    fs::write(&tmp, generated.cert.der())
        .and_then(|()| fs::rename(&tmp, cert_path))
        .with_context(|| format!("writing {}", cert_path.display()))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("plusysmon-tls-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn keeps_its_identity_and_guards_the_key() {
        let dir = temp_dir("keeps");
        let cert = dir.join("tls-cert.der");
        let first = load_or_create(&cert).unwrap();
        let second = load_or_create(&cert).unwrap();
        assert_eq!(first.fingerprint, second.fingerprint);
        let mode = fs::metadata(dir.join("tls-key.der"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn regenerates_after_a_half_finished_first_run() {
        let dir = temp_dir("half");
        let cert = dir.join("tls-cert.der");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("tls-key.der"), b"partial").unwrap();
        load_or_create(&cert).unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_a_damaged_key_rather_than_changing_identity() {
        let dir = temp_dir("damaged");
        let cert = dir.join("tls-cert.der");
        let original = load_or_create(&cert).unwrap().fingerprint;
        fs::write(dir.join("tls-key.der"), b"garbage").unwrap();
        assert!(load_or_create(&cert).is_err());
        assert_eq!(cert_fingerprint(&fs::read(&cert).unwrap()), original);
        fs::remove_dir_all(&dir).unwrap();
    }
}
