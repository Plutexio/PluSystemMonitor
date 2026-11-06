//! Daemon connection, run on its own thread so the Qt event loop never blocks on I/O.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use plusysmon_protocol::{
    self as proto, ClientMsg, DEFAULT_PORT, DaemonInfo, OLDEST_COMPATIBLE, PROTOCOL_VERSION,
    ProcessUpdate, Sample, SensorReading, SensorSettings, ServerMsg, Signal,
    fans::{FanOutput, FanSettings, FanStatus},
};
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_tungstenite::WebSocketStream;
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
        /// The protocol both sides speak: the daemon's, or ours if it's newer.
        version: u32,
        /// What the daemon said about itself; `None` for daemons before 6.
        info: Option<DaemonInfo>,
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
    /// The daemon answered, but one side is too old for the other; no retry.
    Incompatible(DaemonInfo),
    SensorSettings {
        settings: SensorSettings,
        sensors: Vec<SensorReading>,
        error: Option<String>,
    },
    Fans {
        settings: FanSettings,
        outputs: Vec<FanOutput>,
        status: FanStatus,
        error: Option<String>,
    },
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
    GetSensorSettings,
    SetSensorSettings(SensorSettings),
    /// Only for daemons from version 7 ([`proto::FANS_SINCE`]) with fan control.
    GetFans,
    SetFans(FanSettings),
}

enum End {
    Closed,
    Rejected(String),
    Incompatible(DaemonInfo),
    /// A daemon from before version 6 turned our hello down, but speaks a
    /// version we still know; say hello in that one.
    Downgrade(u32),
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
    let mut version = PROTOCOL_VERSION;
    loop {
        if !on_event(Event::Connecting(format!("wss://{authority}"))) {
            return;
        }
        let event = match session(
            &authority,
            token,
            version,
            commands,
            &mut want_processes,
            on_event,
        )
        .await
        {
            Ok(End::Downgrade(older)) => {
                version = older;
                continue;
            }
            Ok(End::Rejected(reason)) => {
                on_event(Event::Rejected(reason));
                return;
            }
            Ok(End::Incompatible(info)) => {
                on_event(Event::Incompatible(info));
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

/// An open, verified connection to a daemon, before the hello.
pub struct Link {
    pub ws: WebSocketStream<TlsStream<TcpStream>>,
    pub fingerprint: String,
}

pub enum OpenError {
    /// Nothing answered at that address.
    Unreachable(anyhow::Error),
    /// Something answered with a certificate we don't trust; nothing was sent.
    Untrusted {
        fingerprint: String,
        pinned: Option<String>,
    },
    /// Something answered, but the connection failed.
    Failed(anyhow::Error),
}

/// TCP, TLS with the pinned certificate (or this machine's own), WebSocket.
pub async fn open(authority: &str) -> Result<Link, OpenError> {
    // Read on every attempt: this machine's daemon makes its certificate on first run.
    let pinned = tls::pinned(authority);
    let accepted: Vec<String> = pinned
        .iter()
        .cloned()
        .chain(tls::local_fingerprint())
        .collect();
    let presented = Arc::new(Mutex::new(None));
    let connector =
        tls::connector(accepted.clone(), presented.clone()).map_err(OpenError::Failed)?;

    let tcp = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(authority))
        .await
        .map_err(|_| OpenError::Unreachable(anyhow::anyhow!("timed out")))?
        .map_err(|e| OpenError::Unreachable(e.into()))?;
    let name = ServerName::try_from(tls::SERVER_NAME).map_err(|e| OpenError::Failed(e.into()))?;
    let stream = match tokio::time::timeout(CONNECT_TIMEOUT, connector.connect(name, tcp)).await {
        Err(_) => return Err(OpenError::Failed(anyhow::anyhow!("TLS handshake timed out"))),
        Ok(Ok(stream)) => stream,
        // A certificate we know failing anyway is no question for the user: its
        // holder couldn't prove it's theirs, or the handshake broke some other way.
        Ok(Err(e)) => {
            return Err(match presented.lock().unwrap().take() {
                Some(fingerprint) if !accepted.contains(&fingerprint) => OpenError::Untrusted {
                    fingerprint,
                    pinned,
                },
                _ => OpenError::Failed(
                    anyhow::Error::from(e).context("TLS handshake failed; is the daemon up to date?"),
                ),
            });
        }
    };
    let fingerprint = presented.lock().unwrap().take().unwrap_or_default();
    let url = format!("wss://{authority}");
    let (ws, _) = tokio::time::timeout(
        CONNECT_TIMEOUT,
        tokio_tungstenite::client_async(url.as_str(), stream),
    )
    .await
    .map_err(|_| OpenError::Failed(anyhow::anyhow!("timed out")))?
    .map_err(|e| OpenError::Failed(e.into()))?;
    Ok(Link { ws, fingerprint })
}

impl Link {
    pub async fn send(&mut self, msg: &ClientMsg) -> Result<()> {
        self.ws.send(Message::binary(proto::encode(msg))).await?;
        Ok(())
    }

    /// The next message, or `None` once the daemon closed the connection.
    pub async fn recv(&mut self) -> Result<Option<ServerMsg>> {
        loop {
            let Some(msg) = self.ws.next().await else {
                return Ok(None);
            };
            match msg? {
                Message::Binary(bytes) => {
                    return Ok(Some(
                        proto::decode(&bytes).context("malformed message from daemon")?,
                    ));
                }
                Message::Close(_) => return Ok(None),
                _ => continue,
            }
        }
    }

    pub async fn hello(&mut self, token: &str, version: u32, probe: bool) -> Result<()> {
        self.send(&ClientMsg::Hello {
            version,
            token: token.to_owned(),
            oldest: OLDEST_COMPATIBLE,
            probe,
        })
        .await
    }
}

/// The daemon version in a rejection by a daemon from before version 6:
/// "protocol version mismatch: daemon 5, client 6".
pub fn rejected_version(reason: &str) -> Option<u32> {
    let rest = reason.strip_prefix("protocol version mismatch: daemon ")?;
    rest.split(',').next()?.trim().parse().ok()
}

/// Why a daemon and this app can't talk, and what to do about it.
pub fn incompatibility(hostname: &str, daemon: u32) -> String {
    if daemon < OLDEST_COMPATIBLE {
        format!(
            "The daemon on {hostname} is too old for this app (protocol {daemon}, this app needs \
             {OLDEST_COMPATIBLE} or newer). It requires an update."
        )
    } else {
        format!(
            "This app is too old for the daemon on {hostname} (protocol {daemon}, this app has \
             {PROTOCOL_VERSION}). Update this computer."
        )
    }
}

async fn session(
    authority: &str,
    token: &str,
    version: u32,
    commands: &mut UnboundedReceiver<Command>,
    want_processes: &mut bool,
    on_event: &mut impl FnMut(Event) -> bool,
) -> Result<End> {
    let mut link = match open(authority).await {
        Ok(link) => link,
        Err(OpenError::Untrusted {
            fingerprint,
            pinned,
        }) => {
            return Ok(End::Untrusted {
                fingerprint,
                pinned,
            });
        }
        Err(OpenError::Unreachable(e) | OpenError::Failed(e)) => return Err(e),
    };
    link.hello(token, version, false).await?;
    if *want_processes {
        link.send(&ClientMsg::Subscribe { processes: true }).await?;
    }

    let mut info: Option<DaemonInfo> = None;
    loop {
        let msg = tokio::select! {
            msg = link.recv() => msg?,
            command = commands.recv() => {
                let msg = match command {
                    Some(Command::Subscribe { processes }) => {
                        *want_processes = processes;
                        ClientMsg::Subscribe { processes }
                    }
                    Some(Command::Signal { pid, signal }) => ClientMsg::Signal { pid, signal },
                    Some(Command::Asset { key }) => ClientMsg::Asset { key },
                    Some(Command::GetSensorSettings) => ClientMsg::GetSensorSettings,
                    Some(Command::SetSensorSettings(settings)) => {
                        ClientMsg::SetSensorSettings(settings)
                    }
                    Some(Command::GetFans) => ClientMsg::GetFans,
                    Some(Command::SetFans(settings)) => ClientMsg::SetFans(settings),
                    // The UI dropped its sender; `stop` ends this thread shortly.
                    None => std::future::pending().await,
                };
                link.send(&msg).await?;
                continue;
            }
        };
        let Some(msg) = msg else { break };
        let event = match msg {
            ServerMsg::Info(daemon) => {
                if !daemon.compatible {
                    return Ok(End::Incompatible(daemon));
                }
                info = Some(daemon);
                continue;
            }
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
                fingerprint: link.fingerprint.clone(),
                version: info.as_ref().map_or(version, |i| i.version.min(PROTOCOL_VERSION)),
                info: info.clone(),
            },
            ServerMsg::Sample(sample) => Event::Sample(sample),
            ServerMsg::Processes(update) => Event::Processes(update),
            ServerMsg::SignalResult { pid, signal, error } => {
                Event::SignalResult { pid, signal, error }
            }
            ServerMsg::Asset { key, format, data } => Event::Asset { key, format, data },
            ServerMsg::SensorSettings {
                settings,
                sensors,
                error,
            } => Event::SensorSettings {
                settings,
                sensors,
                error,
            },
            ServerMsg::Fans {
                settings,
                outputs,
                status,
                error,
            } => Event::Fans {
                settings,
                outputs,
                status,
                error,
            },
            // The update panel follows updates on connections of its own.
            ServerMsg::Updates(_) | ServerMsg::UpdateLog(_) => continue,
            ServerMsg::Error(reason) => {
                return Ok(match rejected_version(&reason) {
                    Some(older) if older >= OLDEST_COMPATIBLE && older < version => {
                        End::Downgrade(older)
                    }
                    Some(older) => End::Rejected(incompatibility(authority, older)),
                    None => End::Rejected(reason),
                });
            }
        };
        if !on_event(event) {
            bail!("frontend went away");
        }
    }
    Ok(End::Closed)
}

/// The token to send: the one given, or this machine's own for a local daemon.
pub fn resolve_token(address: &str, token: &str) -> String {
    let token = token.trim();
    if token.is_empty() && is_local(address) {
        return plusysmon_protocol::token_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|t| t.trim().to_owned())
            .unwrap_or_default();
    }
    token.to_owned()
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
    fn reads_old_rejections() {
        assert_eq!(
            rejected_version("protocol version mismatch: daemon 5, client 6"),
            Some(5)
        );
        assert_eq!(rejected_version("invalid token"), None);
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
