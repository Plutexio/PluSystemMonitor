mod collect;
mod server;
mod token;

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use plusysmon_protocol::DEFAULT_PORT;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

const USAGE: &str = "\
Usage: plusysmond [OPTIONS]

Options:
  --listen <ADDR>        Address to listen on [default: 127.0.0.1:7878]
                         Use 0.0.0.0:7878 to accept connections from other machines.
  --interval-ms <MS>     Sampling interval [default: 1000]
  --history-secs <SECS>  How much history to keep for new clients [default: 1800]
  -h, --help             Print this help";

struct Args {
    listen: SocketAddr,
    interval_ms: u32,
    history_secs: u32,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        listen: SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT)),
        interval_ms: 1000,
        history_secs: 1800,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().with_context(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--listen" => args.listen = value()?.parse().context("--listen")?,
            "--interval-ms" => args.interval_ms = value()?.parse().context("--interval-ms")?,
            "--history-secs" => args.history_secs = value()?.parse().context("--history-secs")?,
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => bail!("unknown argument {other}\n\n{USAGE}"),
        }
    }
    if args.interval_ms < 100 {
        bail!("--interval-ms must be at least 100");
    }
    Ok(args)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_target(false).init();
    let args = parse_args()?;

    let token_path = plusysmon_protocol::token_path().context("cannot determine config dir")?;
    let token = token::load_or_create(&token_path)?;
    info!("access token: {}", token_path.display());

    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|h| h.trim().to_owned())
        .unwrap_or_else(|_| "unknown".to_owned());
    let capacity = (args.history_secs as usize * 1000 / args.interval_ms as usize).max(1);

    let shared = Arc::new(server::Shared {
        hostname,
        interval_ms: args.interval_ms,
        token,
        history: Mutex::new(VecDeque::with_capacity(capacity)),
        samples: broadcast::channel(16).0,
    });

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = TcpListener::bind(args.listen)
                .await
                .with_context(|| format!("binding {}", args.listen))?;
            info!("listening on ws://{}", args.listen);
            if !args.listen.ip().is_loopback() {
                warn!("accepting remote connections; traffic is not encrypted yet");
            }

            tokio::spawn(sample_loop(shared.clone(), capacity, args.interval_ms));
            tokio::select! {
                res = server::serve(listener, shared) => res,
                _ = shutdown_signal() => {
                    info!("shutting down");
                    Ok(())
                }
            }
        })
}

async fn sample_loop(shared: Arc<server::Shared>, capacity: usize, interval_ms: u32) {
    let mut collector = match collect::Collector::new() {
        Ok(c) => c,
        Err(e) => {
            error!("cannot start collector: {e:#}");
            std::process::exit(1);
        }
    };
    let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms.into()));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // The first tick fires immediately, before there is a delta to measure.
    ticker.tick().await;

    loop {
        ticker.tick().await;
        let sample = match collector.sample() {
            Ok(s) => s,
            Err(e) => {
                warn!("sampling failed: {e:#}");
                continue;
            }
        };
        let mut history = shared.history.lock().unwrap();
        if history.len() == capacity {
            history.pop_front();
        }
        history.push_back(sample.clone());
        // Err only means nobody is connected right now.
        let _ = shared.samples.send(sample);
    }
}

async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = signal(SignalKind::terminate()).expect("installing SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}
