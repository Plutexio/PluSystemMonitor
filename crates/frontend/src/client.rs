//! Daemon connection, run on its own thread so the Qt event loop never blocks on I/O.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use plusysmon_protocol::{
    self as proto, ClientMsg, DEFAULT_PORT, PROTOCOL_VERSION, ProcessUpdate, Sample, ServerMsg,
    Signal,
};
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_tungstenite::tungstenite::Message;

use crate::tls;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_secs(2);

pub enum Event {
    Connecting(String),
    Connected {
        hostname: String,
        interval_ms: u32,
        uid: u32,
        history: Vec<Sample>,
        fingerprint: String,
    },
    Sample(Sample),
    Processes(ProcessUpdate),
    SignalResult {
        pid: u32,
        signal: Signal,
        error: Option<String>,
    },
    Asset {
        key: String,
        format: String,
        data: Vec<u8>,
    },
    /// The connection dropped; a reconnect is scheduled.
    Lost(String),
    /// The daemon refused us (bad token, version mismatch); no retry.
    Rejected(String),
    /// The daemon's certificate isn't one we trust; no retry until the user
    /// decides. `pinned` is the one it should have had, if we knew this address.
    Untrusted {
        authority: String,
        fingerprint: String,
        pinned: Option<String>,
    },
}

/// Requests from the UI. They survive reconnects: subscriptions are re-sent
/// to every new session.
pub enum Command {
    Subscribe { processes: bool },
    Signal { pid: u32, signal: Signal },
    Asset { key: String },
}

enum End {
    Closed,
    Rejected(String),
    Untrusted {
        fingerprint: String,
        pinned: Option<String>,
    },
}

/// Connects to `address` and keeps reconnecting until `stop` is notified,
/// the daemon rejects us, or `on_event` returns false (its receiver is gone).
pub fn spawn(
    address: String,
    token: String,
    stop: Arc<Notify>,
    mut commands: UnboundedReceiver<Command>,
    mut on_event: impl FnMut(Event) -> bool + Send + 'static,
) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("building client runtime");
        runtime.block_on(async {
            tokio::select! {
                _ = stop.notified() => {}
                _ = run(&address, &token, &mut commands, &mut on_event) => {}
            }
        });
    });
}

async fn run(
    address: &str,
    token: &str,
    commands: &mut UnboundedReceiver<Command>,
    on_event: &mut impl FnMut(Event) -> bool,
) {
    let authority = authority(address);
    let mut want_processes = false;
    loop {
        if !on_event(Event::Connecting(format!("wss://{authority}"))) {
            return;
        }
        let event = match session(&authority, token, commands, &mut want_processes, on_event).await
        {
            Ok(End::Rejected(reason)) => {
                on_event(Event::Rejected(reason));
                return;
            }
            Ok(End::Untrusted {
                fingerprint,
                pinned,
            }) => {
                on_event(Event::Untrusted {
                    authority,
                    fingerprint,
                    pinned,
                });
                return;
            }
            Ok(End::Closed) => Event::Lost("connection closed".to_owned()),
            Err(e) => Event::Lost(format!("{e:#}")),
        };
        if !on_event(event) {
            return;
        }
        tokio::time::sleep(RETRY_DELAY).await;
    }
}

async fn session(
    authority: &str,
    token: &str,
    commands: &mut UnboundedReceiver<Command>,
    want_processes: &mut bool,
    on_event: &mut impl FnMut(Event) -> bool,
) -> Result<End> {
    // Read on every attempt: this machine's daemon makes its certificate on first run.
    let pinned = tls::pinned(authority);
    let accepted: Vec<String> = pinned
        .iter()
        .cloned()
        .chain(tls::local_fingerprint())
        .collect();
    let presented = Arc::new(Mutex::new(None));
    let connector = tls::connector(accepted.clone(), presented.clone())?;

    let handshake = async {
        let tcp = TcpStream::connect(authority).await?;
        let name = ServerName::try_from(tls::SERVER_NAME)?;
        anyhow::Ok(connector.connect(name, tcp).await)
    };
    let stream = match tokio::time::timeout(CONNECT_TIMEOUT, handshake)
        .await
        .context("timed out")??
    {
        Ok(stream) => stream,
        // A certificate we know failing anyway is no question for the user: its
        // holder couldn't prove it's theirs, or the handshake broke some other way.
        Err(e) => {
            return match presented.lock().unwrap().take() {
                Some(fingerprint) if !accepted.contains(&fingerprint) => Ok(End::Untrusted {
                    fingerprint,
                    pinned,
                }),
                _ => Err(e).context("TLS handshake failed; is the daemon up to date?"),
            };
        }
    };
    let fingerprint = presented.lock().unwrap().take().unwrap_or_default();
    let url = format!("wss://{authority}");
    let (mut ws, _) = tokio::time::timeout(
        CONNECT_TIMEOUT,
        tokio_tungstenite::client_async(url.as_str(), stream),
    )
    .await
    .context("timed out")??;
    let hello = ClientMsg::Hello {
        version: PROTOCOL_VERSION,
        token: token.to_owned(),
    };
    ws.send(Message::binary(proto::encode(&hello))).await?;
    if *want_processes {
        let subscribe = ClientMsg::Subscribe { processes: true };
        ws.send(Message::binary(proto::encode(&subscribe))).await?;
    }

    loop {
        let msg = tokio::select! {
            msg = ws.next() => msg,
            command = commands.recv() => {
                let msg = match command {
                    Some(Command::Subscribe { processes }) => {
                        *want_processes = processes;
                        ClientMsg::Subscribe { processes }
                    }
                    Some(Command::Signal { pid, signal }) => ClientMsg::Signal { pid, signal },
                    Some(Command::Asset { key }) => ClientMsg::Asset { key },
                    // The UI dropped its sender; `stop` ends this thread shortly.
                    None => std::future::pending().await,
                };
                ws.send(Message::binary(proto::encode(&msg))).await?;
                continue;
            }
        };
        let Some(msg) = msg else { break };
        let bytes = match msg? {
            Message::Binary(bytes) => bytes,
            Message::Close(_) => break,
            _ => continue,
        };
        let event = match proto::decode(&bytes).context("malformed message from daemon")? {
            ServerMsg::Welcome {
                hostname,
                interval_ms,
                uid,
                history,
            } => Event::Connected {
                hostname,
                interval_ms,
                uid,
                history,
                fingerprint: fingerprint.clone(),
            },
            ServerMsg::Sample(sample) => Event::Sample(sample),
            ServerMsg::Processes(update) => Event::Processes(update),
            ServerMsg::SignalResult { pid, signal, error } => {
                Event::SignalResult { pid, signal, error }
            }
            ServerMsg::Asset { key, format, data } => Event::Asset { key, format, data },
            ServerMsg::Error(reason) => return Ok(End::Rejected(reason)),
        };
        if !on_event(event) {
            bail!("frontend went away");
        }
    }
    Ok(End::Closed)
}

/// `host:port` of an address given as `host`, `host:port` or a URL. A URL's
/// scheme doesn't matter: connections are always encrypted, whatever it says.
pub fn authority(address: &str) -> String {
    let rest = address.split_once("://").map_or(address, |(_, rest)| rest);
    let host = rest.split('/').next().unwrap_or_default();
    let has_port = host
        .rsplit_once(':')
        .is_some_and(|(_, port)| port.parse::<u16>().is_ok())
        && !host.ends_with(']');
    if has_port {
        host.to_owned()
    } else {
        format!("{host}:{DEFAULT_PORT}")
    }
}

pub fn is_local(address: &str) -> bool {
    let authority = authority(address);
    let host = authority.rsplit_once(':').map_or("", |(host, _)| host);
    ["localhost", "127.0.0.1", "[::1]"].contains(&host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_authority() {
        assert_eq!(authority("192.168.1.10"), "192.168.1.10:7878");
        assert_eq!(authority("192.168.1.10:9000"), "192.168.1.10:9000");
        assert_eq!(authority("[::1]"), "[::1]:7878");
        assert_eq!(authority("[::1]:9000"), "[::1]:9000");
        assert_eq!(authority("ws://box:1"), "box:1");
        assert_eq!(authority("wss://box/"), "box:7878");
    }

    #[test]
    fn detects_local() {
        assert!(is_local("127.0.0.1"));
        assert!(is_local("localhost:7878"));
        assert!(is_local("[::1]"));
        assert!(!is_local("192.168.1.10"));
    }

    /// The local token must not go to a machine that merely looks local.
    #[test]
    fn lookalikes_are_not_local() {
        assert!(!is_local("127.0.0.1.example.com"));
        assert!(!is_local("localhost.example.com:7878"));
    }
}
