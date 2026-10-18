//! Daemon connection, run on its own thread so the Qt event loop never blocks on I/O.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use plusysmon_protocol::{
    self as proto, ClientMsg, DEFAULT_PORT, PROTOCOL_VERSION, ProcessUpdate, Sample, ServerMsg,
    Signal,
};
use tokio::sync::Notify;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::tungstenite::Message;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_secs(2);

pub enum Event {
    Connecting(String),
    Connected {
        hostname: String,
        interval_ms: u32,
        uid: u32,
        history: Vec<Sample>,
    },
    Sample(Sample),
    Processes(ProcessUpdate),
    SignalResult {
        pid: u32,
        signal: Signal,
        error: Option<String>,
    },
    /// The connection dropped; a reconnect is scheduled.
    Lost(String),
    /// The daemon refused us (bad token, version mismatch); no retry.
    Rejected(String),
}

/// Requests from the UI. They survive reconnects: subscriptions are re-sent
/// to every new session.
pub enum Command {
    Subscribe { processes: bool },
    Signal { pid: u32, signal: Signal },
}

enum End {
    Closed,
    Rejected(String),
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
    let url = to_url(address);
    let mut want_processes = false;
    loop {
        if !on_event(Event::Connecting(url.clone())) {
            return;
        }
        let event = match session(&url, token, commands, &mut want_processes, on_event).await {
            Ok(End::Rejected(reason)) => {
                on_event(Event::Rejected(reason));
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
    url: &str,
    token: &str,
    commands: &mut UnboundedReceiver<Command>,
    want_processes: &mut bool,
    on_event: &mut impl FnMut(Event) -> bool,
) -> Result<End> {
    let (mut ws, _) = tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url))
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
            },
            ServerMsg::Sample(sample) => Event::Sample(sample),
            ServerMsg::Processes(update) => Event::Processes(update),
            ServerMsg::SignalResult { pid, signal, error } => {
                Event::SignalResult { pid, signal, error }
            }
            ServerMsg::Error(reason) => return Ok(End::Rejected(reason)),
        };
        if !on_event(event) {
            bail!("frontend went away");
        }
    }
    Ok(End::Closed)
}

/// Accepts `host`, `host:port` or a full `ws://` URL.
fn to_url(address: &str) -> String {
    if address.contains("://") {
        return address.to_owned();
    }
    let has_port = address
        .rsplit_once(':')
        .is_some_and(|(_, port)| port.parse::<u16>().is_ok())
        && !address.ends_with(']');
    if has_port {
        format!("ws://{address}")
    } else {
        format!("ws://{address}:{DEFAULT_PORT}")
    }
}

pub fn is_local(address: &str) -> bool {
    let host = to_url(address);
    let host = host.split("://").nth(1).unwrap_or_default();
    ["localhost", "127.0.0.1", "[::1]"]
        .iter()
        .any(|local| host.starts_with(local))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_urls() {
        assert_eq!(to_url("192.168.1.10"), "ws://192.168.1.10:7878");
        assert_eq!(to_url("192.168.1.10:9000"), "ws://192.168.1.10:9000");
        assert_eq!(to_url("[::1]"), "ws://[::1]:7878");
        assert_eq!(to_url("ws://box:1"), "ws://box:1");
    }

    #[test]
    fn detects_local() {
        assert!(is_local("127.0.0.1"));
        assert!(is_local("localhost:7878"));
        assert!(!is_local("192.168.1.10"));
    }
}
