mod collect;
mod server;
mod table;
mod tls;
mod token;
mod update;
#[cfg(feature = "widgets")]
mod widgets;

use std::collections::VecDeque;
use std::io::IsTerminal;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use plusysmon_protocol::{DEFAULT_PORT, SensorSettings};
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
  --no-widgets           Don't offer data to desktop widgets on the session bus
  --no-updates           Don't update from the git clone install.sh ran from, even
                         when a frontend asks (install.sh --no-updates does the same)
  --fingerprint          Print the TLS certificate's fingerprint, the one frontends
                         ask you to compare when first connecting, and exit
  --version              Print the protocol version and build, and exit
  -h, --help             Print this help";

struct Args {
    listen: SocketAddr,
    interval_ms: u32,
    history_secs: u32,
    print_fingerprint: bool,
    no_updates: bool,
    no_widgets: bool,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        listen: SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT)),
        interval_ms: 1000,
        history_secs: 1800,
        print_fingerprint: false,
        no_updates: false,
        no_widgets: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().with_context(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--listen" => args.listen = value()?.parse().context("--listen")?,
            "--interval-ms" => args.interval_ms = value()?.parse().context("--interval-ms")?,
            "--history-secs" => args.history_secs = value()?.parse().context("--history-secs")?,
            "--fingerprint" => args.print_fingerprint = true,
            "--no-updates" => args.no_updates = true,
            "--no-widgets" => args.no_widgets = true,
            "--version" => {
                println!(
                    "plusysmond protocol {} (works with {} and newer), build {}",
                    plusysmon_protocol::PROTOCOL_VERSION,
                    plusysmon_protocol::OLDEST_COMPATIBLE,
                    plusysmon_protocol::BUILD_COMMIT
                );
                std::process::exit(0);
            }
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
    // No colour codes when logging to the journal (systemd service) or a file.
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(std::io::stdout().is_terminal())
        .init();
    let args = parse_args()?;

    let cert_path = plusysmon_protocol::tls_cert_path().context("cannot determine config dir")?;
    let identity = tls::load_or_create(&cert_path)?;
    if args.print_fingerprint {
        println!("{}", identity.fingerprint);
        return Ok(());
    }

    let token_path = plusysmon_protocol::token_path().context("cannot determine config dir")?;
    let token = token::load_or_create(&token_path)?;
    info!("access token: {}", token_path.display());

    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|h| h.trim().to_owned())
        .unwrap_or_else(|_| "unknown".to_owned());
    let capacity = (args.history_secs as usize * 1000 / args.interval_ms as usize).max(1);
    info!(
        "protocol {}, build {}",
        plusysmon_protocol::PROTOCOL_VERSION,
        plusysmon_protocol::BUILD_COMMIT
    );
    let sensor_settings = load_sensor_settings();

    let shared = Arc::new(server::Shared {
        hostname,
        interval_ms: args.interval_ms,
        token,
        // SAFETY: getuid cannot fail.
        uid: unsafe { libc::getuid() },
        history: Mutex::new(VecDeque::with_capacity(capacity)),
        samples: broadcast::channel(16).0,
        processes: Mutex::default(),
        assets: collect::Assets::default(),
        process_updates: broadcast::channel(16).0,
        process_subscribers: AtomicUsize::new(0),
        sensor_settings: Mutex::new(sensor_settings),
        raw_sensors: Mutex::default(),
        updater: update::Updater::new(args.no_updates),
    });

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = TcpListener::bind(args.listen)
                .await
                .with_context(|| format!("binding {}", args.listen))?;
            info!("listening on wss://{}", args.listen);
            info!("certificate fingerprint: {}", identity.fingerprint);

            tokio::spawn(sample_loop(shared.clone(), capacity, args.interval_ms));
            #[cfg(feature = "widgets")]
            if !args.no_widgets {
                tokio::spawn(widgets::run(
                    args.listen,
                    shared.token.clone(),
                    shared.hostname.clone(),
                ));
            }
            #[cfg(not(feature = "widgets"))]
            let _ = args.no_widgets;
            tokio::select! {
                res = server::serve(listener, identity.acceptor, shared) => res,
                _ = shutdown_signal() => {
                    info!("shutting down");
                    Ok(())
                }
            }
        })
}

async fn sample_loop(shared: Arc<server::Shared>, capacity: usize, interval_ms: u32) {
    let mut collector = match collect::Collector::new(shared.assets.clone()) {
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
        let mut sample = match collector.sample() {
            Ok(s) => s,
            Err(e) => {
                warn!("sampling failed: {e:#}");
                continue;
            }
        };
        apply_sensor_settings(&shared, &mut sample);
        let mut history = shared.history.lock().unwrap();
        if history.len() == capacity {
            history.pop_front();
        }
        history.push_back(sample.clone());
        // Err only means nobody is connected right now.
        let _ = shared.samples.send(sample);
        drop(history);

        // Walking /proc costs a few milliseconds, so only do it while someone looks.
        if shared.process_subscribers.load(Ordering::Relaxed) > 0 {
            let (current, new_apps) = collector.sample_processes();
            let mut processes = shared.processes.lock().unwrap();
            let update = processes.update(current, new_apps);
            if !table::is_empty(&update) {
                let _ = shared.process_updates.send(Arc::new(update));
            }
        } else if collector.processes_active() {
            collector.reset_processes();
            shared.processes.lock().unwrap().clear();
        }
    }
}

/// The machine's own sensor names, scales and CPU sources. A broken file is
/// reported and ignored, never fatal.
fn load_sensor_settings() -> SensorSettings {
    let Some(path) = plusysmon_protocol::sensor_settings_path() else {
        return SensorSettings::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return SensorSettings::default();
    };
    match toml::from_str::<SensorSettings>(&text)
        .map_err(|e| e.to_string())
        .and_then(SensorSettings::validated)
    {
        Ok(settings) => {
            info!("sensor settings: {}", path.display());
            settings
        }
        Err(e) => {
            warn!("ignoring {}: {e}", path.display());
            SensorSettings::default()
        }
    }
}

/// Keeps the readings as collected for setting things up, and sends them
/// on as the settings say.
fn apply_sensor_settings(shared: &server::Shared, sample: &mut plusysmon_protocol::Sample) {
    let raw = std::mem::take(&mut sample.sensors);
    let (shown, temperature, power) = shared.sensor_settings.lock().unwrap().apply(&raw);
    sample.sensors = shown;
    // A chosen sensor that went away leaves what was found automatically.
    if temperature.is_some() {
        sample.cpu.temperature = temperature;
    }
    if power.is_some() {
        sample.cpu.power = power;
    }
    *shared.raw_sensors.lock().unwrap() = raw;
}

async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = signal(SignalKind::terminate()).expect("installing SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}
