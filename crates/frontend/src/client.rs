//! Daemon connection, run on its own thread so the Qt event loop never blocks on I/O.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use plusysmon_protocol::{self as proto, ClientMsg, DEFAULT_PORT, PROTOCOL_VERSION, Sample, ServerMsg};
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::Message;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_secs(2);

pub enum Event {
    Connecting(String),
    Connected { hostname: String, interval_ms: u32, history: Vec<Sample> },
    Sample(Sample),
    /// The connection dropped; a reconnect is scheduled.
    Lost(String),
    /// The daemon refused us (bad token, version mismatch); no retry.
    Rejected(String),
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
                _ = run(&address, &token, &mut on_event) => {}
            }
        });
    });
}

async fn run(address: &str, token: &str, on_event: &mut impl FnMut(Event) -> bool) {
    let url = to_url(address);
    loop {
        if !on_event(Event::Connecting(url.clone())) {
            return;
        }
        let event = match session(&url, token, on_event).await {
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

async fn session(url: &str, token: &str, on_event: &mut impl FnMut(Event) -> bool) -> Result<End> {
    let (mut ws, _) = tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url))
        .await
        .context("timed out")??;
    let hello = ClientMsg::Hello { version: PROTOCOL_VERSION, token: token.to_owned() };
    ws.send(Message::binary(proto::encode(&hello))).await?;

    while let Some(msg) = ws.next().await {
        let bytes = match msg? {
            Message::Binary(bytes) => bytes,
            Message::Close(_) => break,
            _ => continue,
        };
        let event = match proto::decode(&bytes).context("malformed message from daemon")? {
            ServerMsg::Welcome { hostname, interval_ms, history } => {
                Event::Connected { hostname, interval_ms, history }
            }
            ServerMsg::Sample(sample) => Event::Sample(sample),
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
    let has_port = address.rsplit_once(':').is_some_and(|(_, port)| port.parse::<u16>().is_ok())
        && !address.ends_with(']');
    if has_port { format!("ws://{address}") } else { format!("ws://{address}:{DEFAULT_PORT}") }
}

pub fn is_local(address: &str) -> bool {
    let host = to_url(address);
    let host = host.split("://").nth(1).unwrap_or_default();
    ["localhost", "127.0.0.1", "[::1]"].iter().any(|local| host.starts_with(local))
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
