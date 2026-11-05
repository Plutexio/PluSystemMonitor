//! Who we'll talk to. Daemons have self-signed certificates, so there is no
//! authority to ask: a daemon is identified by its certificate's fingerprint,
//! remembered the first time the user accepts it, as SSH does with host keys.
//! The handshake fails before anything is sent, so a stranger never sees the token.

use std::collections::BTreeMap;
use std::fs;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use plusysmon_protocol::cert_fingerprint;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::crypto::{self, CryptoProvider};
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_rustls::rustls::version::TLS13;
use tokio_rustls::rustls::{CertificateError, ClientConfig, DigitallySignedStruct, Error};

use crate::paths;

/// Sent as the name to connect to. It means nothing: the fingerprint decides,
/// and this way a hostname that isn't a valid DNS name still works.
pub const SERVER_NAME: &str = "plusysmond";

#[derive(Debug)]
struct PinVerifier {
    accepted: Vec<String>,
    presented: Arc<Mutex<Option<String>>>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let fingerprint = cert_fingerprint(end_entity);
        let accepted = self.accepted.contains(&fingerprint);
        *self.presented.lock().unwrap() = Some(fingerprint);
        if accepted {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(Error::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    // These stay real: they're what proves the peer holds the certificate's key,
    // since the certificate itself is public.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// A connector that only completes handshakes with the `accepted` fingerprints.
/// `presented` receives the fingerprint the peer showed, accepted or not.
pub fn connector(
    accepted: Vec<String>,
    presented: Arc<Mutex<Option<String>>>,
) -> Result<TlsConnector> {
    let provider = Arc::new(crypto::ring::default_provider());
    let verifier = PinVerifier {
        accepted,
        presented,
        provider: provider.clone(),
    };
    let config = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

/// The certificate of this machine's own daemon, if it has one.
pub fn local_fingerprint() -> Option<String> {
    let der = fs::read(plusysmon_protocol::tls_cert_path()?).ok()?;
    Some(cert_fingerprint(&der))
}

/// `known_daemons.toml`: `"host:port" = "fingerprint"`. Kept apart from the
/// preferences, which go through QML and back.
fn read_pins() -> BTreeMap<String, String> {
    paths::known_daemons_file()
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn pinned(authority: &str) -> Option<String> {
    read_pins().remove(authority)
}

pub fn pin(authority: &str, fingerprint: &str) -> Result<()> {
    let mut pins = read_pins();
    pins.insert(authority.to_owned(), fingerprint.to_owned());
    paths::write_atomic(
        &paths::known_daemons_file()?,
        &toml::to_string_pretty(&pins)?,
    )
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::TlsAcceptor;
    use tokio_rustls::rustls::ServerConfig;
    use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
    use tokio_rustls::rustls::sign::{CertifiedKey, SingleCertAndKey};

    use super::*;

    struct Daemon {
        cert: Vec<u8>,
        key: Vec<u8>,
    }

    fn daemon() -> Daemon {
        let made = rcgen::generate_simple_self_signed(["plusysmond".to_owned()]).unwrap();
        Daemon {
            cert: made.cert.der().to_vec(),
            key: made.signing_key.serialize_der(),
        }
    }

    /// Serves `cert` signing with `key`, which need not belong together.
    fn acceptor(cert: &[u8], key: &[u8]) -> TlsAcceptor {
        let provider = Arc::new(crypto::ring::default_provider());
        let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key.to_vec()));
        let signer = provider.key_provider.load_private_key(key).unwrap();
        let certified = CertifiedKey::new(vec![CertificateDer::from(cert.to_vec())], signer);
        let config = ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&TLS13])
            .unwrap()
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(SingleCertAndKey::from(certified)));
        TlsAcceptor::from(Arc::new(config))
    }

    /// Runs a handshake and, if it completes, sends "token". Returns what the
    /// server received and the fingerprint the client saw.
    async fn exchange(server: TlsAcceptor, accepted: Vec<String>) -> (Vec<u8>, Option<String>) {
        let (client_io, server_io) = tokio::io::duplex(16 * 1024);
        let presented = Arc::new(Mutex::new(None));
        let client = connector(accepted, presented.clone()).unwrap();

        let client = async move {
            let name = ServerName::try_from(SERVER_NAME).unwrap();
            if let Ok(mut stream) = client.connect(name, client_io).await {
                stream.write_all(b"token").await.unwrap();
                stream.shutdown().await.unwrap();
                // Stay until the server is done; it still has handshake data to send.
                let _ = stream.read_to_end(&mut Vec::new()).await;
            }
        };
        let server = async move {
            let mut received = Vec::new();
            if let Ok(mut stream) = server.accept(server_io).await {
                let _ = stream.read_to_end(&mut received).await;
            }
            received
        };
        let (_, received) = tokio::join!(client, server);
        let seen = presented.lock().unwrap().clone();
        (received, seen)
    }

    fn run<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn talks_to_the_pinned_daemon() {
        let daemon = daemon();
        let fingerprint = cert_fingerprint(&daemon.cert);
        let (received, seen) = run(exchange(
            acceptor(&daemon.cert, &daemon.key),
            vec![fingerprint.clone()],
        ));
        assert_eq!(received, b"token");
        assert_eq!(seen, Some(fingerprint));
    }

    #[test]
    fn a_stranger_gets_no_token_and_is_reported() {
        let stranger = daemon();
        let (received, seen) = run(exchange(
            acceptor(&stranger.cert, &stranger.key),
            vec!["AB:CD".to_owned()],
        ));
        assert!(received.is_empty());
        assert_eq!(seen, Some(cert_fingerprint(&stranger.cert)));
    }

    /// Certificates are public: replaying a pinned one must not be enough.
    #[test]
    fn an_impostor_with_the_pinned_certificate_gets_no_token() {
        let real = daemon();
        let impostor = daemon();
        let (received, _) = run(exchange(
            acceptor(&real.cert, &impostor.key),
            vec![cert_fingerprint(&real.cert)],
        ));
        assert!(received.is_empty());
    }
}
