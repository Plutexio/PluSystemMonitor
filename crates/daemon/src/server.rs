use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use plusysmon_protocol::{
    self as proto, ClientMsg, PROTOCOL_VERSION, ProcessUpdate, Sample, ServerMsg, Signal,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::error::{Error as WsError, ProtocolError};
use tracing::{info, warn};

use crate::table::ProcessTable;
use crate::token;

const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Shared {
    pub hostname: String,
    pub interval_ms: u32,
    pub token: String,
    pub uid: u32,
    /// Holding this lock while pushing *and* broadcasting a sample (and while
    /// subscribing *and* snapshotting) guarantees new clients see every sample
    /// exactly once. `processes` works the same way.
    pub history: Mutex<VecDeque<Sample>>,
    pub samples: broadcast::Sender<Sample>,
    pub processes: Mutex<ProcessTable>,
    pub process_updates: broadcast::Sender<Arc<ProcessUpdate>>,
    /// The sampler only walks /proc while this is non-zero.
    pub process_subscribers: AtomicUsize,
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

type Socket = WebSocketStream<TcpStream>;

async fn send(ws: &mut Socket, msg: &ServerMsg) -> Result<()> {
    ws.send(Message::binary(proto::encode(msg))).await?;
    Ok(())
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
    let ClientMsg::Hello { version, token } = proto::decode(&bytes).context("malformed hello")?
    else {
        bail!("expected hello as the first message");
    };

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
        send(&mut ws, &ServerMsg::Error(reason.clone())).await.ok();
        ws.close(None).await.ok();
        bail!("rejected: {reason}");
    }

    let (mut samples, history) = {
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
        uid: shared.uid,
        history,
    };
    send(&mut ws, &welcome).await?;

    let mut processes: Option<ProcessSubscription> = None;
    loop {
        tokio::select! {
            sample = samples.recv() => match sample {
                Ok(sample) => send(&mut ws, &ServerMsg::Sample(sample)).await?,
                // A slow client just skips samples; the next one is a full snapshot anyway.
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return Ok(()),
            },
            update = recv_processes(&mut processes) => match update {
                Ok(update) => send(&mut ws, &ServerMsg::Processes((*update).clone())).await?,
                // Deltas can't be skipped; start over from a fresh snapshot.
                Err(RecvError::Lagged(_)) => {
                    let (subscription, snapshot) = ProcessSubscription::start(&shared);
                    processes = Some(subscription);
                    send(&mut ws, &ServerMsg::Processes(snapshot)).await?;
                }
                Err(RecvError::Closed) => return Ok(()),
            },
            incoming = ws.next() => {
                let bytes = match incoming {
                    None | Some(Ok(Message::Close(_))) => return Ok(()),
                    // Clients that simply quit (or crash) don't send a close frame; that's normal.
                    Some(Err(WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake))) => {
                        return Ok(());
                    }
                    Some(Err(e)) => return Err(e.into()),
                    Some(Ok(Message::Binary(bytes))) => bytes,
                    // Pings are answered by tungstenite itself.
                    Some(Ok(_)) => continue,
                };
                match proto::decode(&bytes).context("malformed message")? {
                    ClientMsg::Hello { .. } => bail!("unexpected second hello"),
                    ClientMsg::Subscribe { processes: wanted } => {
                        if wanted && processes.is_none() {
                            let (subscription, snapshot) = ProcessSubscription::start(&shared);
                            processes = Some(subscription);
                            send(&mut ws, &ServerMsg::Processes(snapshot)).await?;
                        } else if !wanted {
                            processes = None;
                        }
                    }
                    ClientMsg::Signal { pid, signal } => {
                        let error = send_signal(pid, signal).err();
                        info!(%peer, pid, ?signal, error = error.as_deref().unwrap_or("none"), "signal");
                        send(&mut ws, &ServerMsg::SignalResult { pid, signal, error }).await?;
                    }
                }
            }
        }
    }
}

/// A client's place in the process stream. Dropping it (unsubscribe or
/// disconnect, however that happens) releases its claim on the sampler.
struct ProcessSubscription {
    shared: Arc<Shared>,
    updates: broadcast::Receiver<Arc<ProcessUpdate>>,
}

impl ProcessSubscription {
    fn start(shared: &Arc<Shared>) -> (Self, ProcessUpdate) {
        shared.process_subscribers.fetch_add(1, Ordering::Relaxed);
        let table = shared.processes.lock().unwrap();
        let subscription = Self {
            shared: shared.clone(),
            updates: shared.process_updates.subscribe(),
        };
        (subscription, table.snapshot())
    }
}

impl Drop for ProcessSubscription {
    fn drop(&mut self) {
        self.shared
            .process_subscribers
            .fetch_sub(1, Ordering::Relaxed);
    }
}

async fn recv_processes(
    subscription: &mut Option<ProcessSubscription>,
) -> Result<Arc<ProcessUpdate>, RecvError> {
    match subscription {
        Some(subscription) => subscription.updates.recv().await,
        None => std::future::pending().await,
    }
}

fn send_signal(pid: u32, signal: Signal) -> Result<(), String> {
    // 0 and negative PIDs address process groups, and -1 means "everything we
    // can reach"; PID 1 is init. None of those should ever come from a click.
    let Ok(target) = i32::try_from(pid) else {
        return Err("invalid PID".to_owned());
    };
    if target <= 1 {
        return Err("refusing to signal this process".to_owned());
    }
    let number = match signal {
        Signal::Terminate => libc::SIGTERM,
        Signal::Kill => libc::SIGKILL,
        Signal::Stop => libc::SIGSTOP,
        Signal::Continue => libc::SIGCONT,
    };
    // SAFETY: kill has no memory-safety preconditions.
    if unsafe { libc::kill(target, number) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_dangerous_pids() {
        assert!(send_signal(0, Signal::Kill).is_err());
        assert!(send_signal(1, Signal::Kill).is_err());
        assert!(send_signal(u32::MAX, Signal::Kill).is_err());
    }

    #[test]
    fn signals_a_child() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        send_signal(child.id(), Signal::Terminate).unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());
    }
}
