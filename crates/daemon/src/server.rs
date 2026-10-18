use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use plusysmon_protocol::{self as proto, ClientMsg, PROTOCOL_VERSION, Sample, ServerMsg};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::error::{Error as WsError, ProtocolError};
use tracing::{info, warn};

use crate::token;

const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Shared {
    pub hostname: String,
    pub interval_ms: u32,
    pub token: String,
    /// Holding this lock while pushing *and* broadcasting a sample (and while
    /// subscribing *and* snapshotting) guarantees new clients see every sample
    /// exactly once.
    pub history: Mutex<VecDeque<Sample>>,
    pub samples: broadcast::Sender<Sample>,
}

pub async fn serve(listener: TcpListener, shared: Arc<Shared>) -> Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let shared = shared.clone();
        tokio::spawn(async move {
            match handle(stream, peer, shared).await {
                Ok(()) => info!(%peer, "client disconnected"),
                Err(e) => warn!(%peer, "client dropped: {e:#}"),
            }
        });
    }
}

async fn handle(stream: TcpStream, peer: SocketAddr, shared: Arc<Shared>) -> Result<()> {
    let mut ws = tokio_tungstenite::accept_async(stream)
        .await
        .context("websocket handshake")?;

    let hello = tokio::time::timeout(HELLO_TIMEOUT, ws.next())
        .await
        .context("no hello received")?
        .context("closed before hello")??;
    let Message::Binary(bytes) = hello else {
        bail!("expected a binary hello frame")
    };
    let ClientMsg::Hello { version, token } = proto::decode(&bytes).context("malformed hello")?;

    let rejection = if version != PROTOCOL_VERSION {
        Some(format!(
            "protocol version mismatch: daemon {PROTOCOL_VERSION}, client {version}"
        ))
    } else if !token::matches(&shared.token, &token) {
        Some("invalid token".to_owned())
    } else {
        None
    };
    if let Some(reason) = rejection {
        ws.send(Message::binary(proto::encode(&ServerMsg::Error(
            reason.clone(),
        ))))
        .await
        .ok();
        ws.close(None).await.ok();
        bail!("rejected: {reason}");
    }

    let (mut rx, history) = {
        let history = shared.history.lock().unwrap();
        (
            shared.samples.subscribe(),
            history.iter().cloned().collect(),
        )
    };
    info!(%peer, "client connected");
    let welcome = ServerMsg::Welcome {
        hostname: shared.hostname.clone(),
        interval_ms: shared.interval_ms,
        history,
    };
    ws.send(Message::binary(proto::encode(&welcome))).await?;

    loop {
        tokio::select! {
            sample = rx.recv() => match sample {
                Ok(sample) => ws.send(Message::binary(proto::encode(&ServerMsg::Sample(sample)))).await?,
                // A slow client just skips samples; the next one is a full snapshot anyway.
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
            },
            incoming = ws.next() => match incoming {
                None | Some(Ok(Message::Close(_))) => return Ok(()),
                // Clients that simply quit (or crash) don't send a close frame; that's normal.
                Some(Err(WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake))) => return Ok(()),
                Some(Err(e)) => return Err(e.into()),
                // Pings are answered by tungstenite itself; clients send nothing else yet.
                Some(Ok(_)) => {}
            },
        }
    }
}
