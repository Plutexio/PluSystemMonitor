//! Fan control (feature `fans`): turns the samples the daemon already takes
//! into fan speeds, every few seconds, and has `plusysfand` set them.
//!
//! Nothing is read from the hardware here: inputs come from the sample
//! history. While no fan is taken from the firmware the loop doesn't even
//! talk to the helper.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use plusysmon_client::metrics::{short_gpu_name, value_of};
use plusysmon_protocol::fans::{
    Curve, FanMode, FanOutput, FanSettings, FanState, FanStatus, HELPER_VERSION, combine,
};
use plusysmon_protocol::{Sample, ServerMsg};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Notify;
use tracing::{info, warn};

use crate::server::Shared;

/// The helper answers at once; anything slower means it's stuck.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(2);
/// After the helper couldn't be reached, try again this much later.
const RETRY: Duration = Duration::from_secs(10);
/// Ticks without an input before a fan goes back to the firmware.
const MISSING_INPUT_TICKS: u32 = 3;

pub struct Fans {
    pub settings: Mutex<FanSettings>,
    /// The controlled fans as of the last tick, for samples.
    pub states: Mutex<Vec<FanState>>,
    /// Wakes the loop when the settings change.
    changed: Notify,
    helper: PathBuf,
}

impl Fans {
    pub fn new(helper: PathBuf) -> Self {
        Self {
            settings: Mutex::new(load()),
            states: Mutex::default(),
            changed: Notify::new(),
            helper,
        }
    }
}

/// The settings saved in `fans.toml`. A broken file is reported and leaves
/// every fan to the firmware.
fn load() -> FanSettings {
    let Some(path) = plusysmon_protocol::fan_settings_path() else {
        return FanSettings::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return FanSettings::default();
    };
    match toml::from_str::<FanSettings>(&text)
        .map_err(|e| e.to_string())
        .and_then(FanSettings::validated)
    {
        Ok(settings) => {
            info!("fan control: {}", path.display());
            settings
        }
        Err(e) => {
            warn!("ignoring {}: {e}", path.display());
            FanSettings::default()
        }
    }
}

/// Saves first: settings only take effect once they'd survive a restart.
pub fn store(shared: &Shared, settings: FanSettings) -> Result<()> {
    let settings = settings.validated().map_err(anyhow::Error::msg)?;
    let path = plusysmon_protocol::fan_settings_path().context("cannot determine config dir")?;
    let text = format!(
        "# Fan control for this machine, as set in PluSystemMonitor.\n{}",
        toml::to_string_pretty(&settings)?
    );
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))?;
    *shared.fans.settings.lock().unwrap() = settings;
    shared.fans.changed.notify_one();
    Ok(())
}

/// The answer to `GetFans`: the settings, and the fans the helper offers.
pub async fn describe(shared: &Shared, error: Option<String>) -> ServerMsg {
    let settings = shared.fans.settings.lock().unwrap().clone();
    let latest = shared.history.lock().unwrap().back().cloned();
    let (outputs, status) = match list_outputs(&shared.fans.helper).await {
        Ok(raw) => (
            raw.iter()
                .map(|o| name_output(o, latest.as_ref()))
                .collect(),
            FanStatus {
                helper: true,
                problem: String::new(),
            },
        ),
        Err(problem) => (
            Vec::new(),
            FanStatus {
                helper: false,
                problem,
            },
        ),
    };
    ServerMsg::Fans {
        settings,
        outputs,
        status,
        error,
    }
}

async fn list_outputs(path: &Path) -> Result<Vec<RawOutput>, String> {
    Helper::connect(path).await?.list().await
}

/// A fan as the helper lists it.
#[derive(Debug, Clone, PartialEq)]
struct RawOutput {
    id: String,
    driver: String,
    percent: Option<f64>,
    min_percent: f64,
    rpm_sensor: Option<String>,
}

impl RawOutput {
    fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split('\t').collect();
        let ["out", id, driver, percent, min, rpm] = fields.as_slice() else {
            return None;
        };
        Some(Self {
            id: id.to_string(),
            driver: driver.to_string(),
            percent: percent.parse().ok(),
            min_percent: min.parse().unwrap_or(0.0),
            rpm_sensor: (*rpm != "-").then(|| rpm.to_string()),
        })
    }
}

/// Names a fan the way the sensor list names its chip and RPM sensor:
/// "Motherboard Fan 2", "RTX 5070 Ti Fan 1".
fn name_output(raw: &RawOutput, sample: Option<&Sample>) -> FanOutput {
    let sensors = sample.map_or(&[][..], |s| &s.sensors[..]);
    let rpm = raw
        .rpm_sensor
        .as_ref()
        .and_then(|id| sensors.iter().find(|s| &s.id == id));
    let (device, number) = match raw.id.strip_prefix("nvidia.") {
        Some(rest) => (
            short_gpu_name(&raw.driver).to_owned(),
            rest.rsplit_once(".fan")
                .and_then(|(_, n)| n.parse::<u32>().ok())
                .map(|n| n + 1),
        ),
        None => {
            let (chip, pwm) = raw.id.rsplit_once('.').unwrap_or((&raw.id, ""));
            let device = rpm
                .map(|r| r.device.clone())
                .or_else(|| {
                    // The chip's other sensors know what it is.
                    let prefix = format!("{chip}.");
                    sensors
                        .iter()
                        .find(|s| s.id.starts_with(&prefix))
                        .map(|s| s.device.clone())
                })
                .unwrap_or_else(|| match raw.driver.as_str() {
                    "amdgpu" => amd_gpu_name(sample),
                    driver => crate::collect::device_name(driver),
                });
            (device, pwm.strip_prefix("pwm").and_then(|n| n.parse().ok()))
        }
    };
    let label = match (rpm, number) {
        (Some(rpm), _) => rpm.label.clone(),
        (None, Some(n)) => format!("Fan {n}"),
        (None, None) => "Fan".to_owned(),
    };
    FanOutput {
        id: raw.id.clone(),
        name: format!("{device} {label}"),
        device,
        rpm_source: rpm.map(|r| format!("sensor.{}", r.id)),
        percent: raw.percent,
        min_percent: raw.min_percent,
    }
}

fn amd_gpu_name(sample: Option<&Sample>) -> String {
    let amd: Vec<_> = sample
        .map(|s| s.gpus.iter().filter(|g| g.vendor == "AMD").collect())
        .unwrap_or_default();
    match amd.as_slice() {
        [gpu] => short_gpu_name(&gpu.name).to_owned(),
        _ => "AMD GPU".to_owned(),
    }
}

/// A connection to `plusysfand`.
struct Helper {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl Helper {
    async fn connect(path: &Path) -> Result<Self, String> {
        let stream = UnixStream::connect(path)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused => {
                    "The fan helper (plusysfand) isn't running. Install fan control with \
                 install.sh --fans, or start it: sudo systemctl start plusysfand"
                        .to_owned()
                }
                _ => format!("Can't reach the fan helper at {}: {e}", path.display()),
            })?;
        let (reader, writer) = stream.into_split();
        let mut helper = Self {
            reader: BufReader::new(reader),
            writer,
        };
        let hello = helper.ask(&format!("hello {HELPER_VERSION}")).await?;
        let version: u32 = hello
            .strip_prefix("ok plusysfand ")
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("The fan helper answered oddly: {hello}"))?;
        if version < HELPER_VERSION {
            return Err(
                "The fan helper is older than this daemon; run install.sh --fans again".to_owned(),
            );
        }
        Ok(helper)
    }

    async fn send(&mut self, line: &str) -> Result<(), String> {
        let gone = |e: std::io::Error| format!("The fan helper went away: {e}");
        self.writer.write_all(line.as_bytes()).await.map_err(gone)?;
        self.writer.write_all(b"\n").await.map_err(gone)
    }

    async fn read_line(&mut self) -> Result<String, String> {
        let mut line = String::new();
        match tokio::time::timeout(ANSWER_TIMEOUT, self.reader.read_line(&mut line)).await {
            Ok(Ok(0)) => Err("The fan helper closed the connection".to_owned()),
            Ok(Ok(_)) => Ok(line.trim_end().to_owned()),
            Ok(Err(e)) => Err(format!("The fan helper went away: {e}")),
            Err(_) => Err("The fan helper stopped answering".to_owned()),
        }
    }

    async fn ask(&mut self, line: &str) -> Result<String, String> {
        self.send(line).await?;
        self.read_line().await
    }

    /// `Ok(Err(why))` is the helper turning a request down; `Err` a lost connection.
    async fn command(&mut self, line: &str) -> Result<Result<(), String>, String> {
        let answer = self.ask(line).await?;
        Ok(match answer.strip_prefix("err ") {
            Some(why) => Err(why.to_owned()),
            None if answer == "ok" => Ok(()),
            None => Err(answer),
        })
    }

    async fn list(&mut self) -> Result<Vec<RawOutput>, String> {
        self.send("list").await?;
        let mut outputs = Vec::new();
        loop {
            let line = self.read_line().await?;
            if line == "end" {
                return Ok(outputs);
            }
            outputs.extend(RawOutput::parse(&line));
        }
    }
}

/// What a curve remembers between ticks.
#[derive(Debug, Default, Clone, Copy)]
struct Held {
    /// The speed the curve gives now, and the input it rose at.
    speed: f64,
    input: f64,
}

/// What a fan remembers between ticks.
#[derive(Debug, Default)]
struct FanMemory {
    /// What was sent last.
    percent: Option<f64>,
    input: Option<f64>,
    missing: u32,
}

/// The working part of the loop, apart from the helper, so it can be tested.
#[derive(Default)]
struct Controller {
    held: HashMap<String, Held>,
    fans: HashMap<String, FanMemory>,
}

/// What one tick decided for one fan.
#[derive(Debug, PartialEq)]
enum Decision {
    Set(f64),
    /// Back to the firmware (no input for too long).
    Auto(String),
}

impl Controller {
    /// Works out every controlled fan's speed from `history` (oldest first).
    fn tick(
        &mut self,
        settings: &FanSettings,
        history: &[Sample],
        sample_ms: u32,
        elapsed: Duration,
    ) -> Vec<(String, Decision, Option<f64>)> {
        // Each curve once, however many fans follow it.
        let mut curves: HashMap<&str, Option<(f64, f64)>> = HashMap::new();
        for curve in &settings.curves {
            let speed = input(curve, history, sample_ms).map(|input| {
                let target = curve.speed_at(input);
                let held = self.held.entry(curve.id.clone()).or_insert(Held {
                    speed: target,
                    input,
                });
                // Up at once; down only once the input fell clear of where it rose.
                if target >= held.speed || input <= held.input - curve.hysteresis {
                    *held = Held {
                        speed: target,
                        input,
                    };
                }
                (held.speed, input)
            });
            curves.insert(&curve.id, speed);
        }
        self.held.retain(|id, _| curves.contains_key(id.as_str()));

        let mut decisions = Vec::new();
        for (id, fan) in &settings.fans {
            let memory = self.fans.entry(id.clone()).or_default();
            let (wanted, curve, input) = match fan.mode {
                FanMode::Auto => continue,
                FanMode::Fixed => (Some(fan.percent), None, None),
                FanMode::Curve => {
                    let curve = settings.curve(&fan.curve);
                    let found = curves.get(fan.curve.as_str()).copied().flatten();
                    (
                        found.map(|(speed, _)| fan.limit(speed)),
                        curve,
                        found.map(|(_, i)| i),
                    )
                }
            };
            let Some(wanted) = wanted else {
                memory.missing += 1;
                let decision = match memory.percent {
                    // A blip: hold the speed a little while.
                    Some(held) if memory.missing < MISSING_INPUT_TICKS => Decision::Set(held),
                    _ => {
                        memory.percent = None;
                        Decision::Auto("None of the curve's sources is there".to_owned())
                    }
                };
                decisions.push((id.clone(), decision, None));
                continue;
            };
            memory.missing = 0;
            memory.input = input;
            let percent = match (memory.percent, curve) {
                (Some(previous), Some(curve)) if wanted != 0.0 => {
                    step(previous, wanted, curve, elapsed)
                }
                _ => wanted,
            };
            memory.percent = Some(percent);
            decisions.push((id.clone(), Decision::Set(percent), input));
        }
        self.fans.retain(|id, _| {
            settings
                .fans
                .get(id)
                .is_some_and(|f| f.mode != FanMode::Auto)
        });
        decisions
    }
}

/// From `previous` towards `wanted`, no faster than the curve allows.
fn step(previous: f64, wanted: f64, curve: &Curve, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if wanted > previous && curve.step_up > 0.0 {
        wanted.min(previous + curve.step_up * secs)
    } else if wanted < previous && curve.step_down > 0.0 {
        wanted.max(previous - curve.step_down * secs)
    } else {
        wanted
    }
}

/// The curve's input: its sources combined, averaged over `average_secs`.
fn input(curve: &Curve, history: &[Sample], sample_ms: u32) -> Option<f64> {
    let wanted = (curve.average_secs as usize * 1000 / sample_ms.max(1) as usize).max(1);
    let values: Vec<f64> = history
        .iter()
        .rev()
        .take(wanted)
        .filter_map(|sample| {
            let values: Vec<f64> = curve
                .sources
                .iter()
                .filter_map(|id| value_of(sample, id))
                .collect();
            combine(curve.combine, &values)
        })
        .collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

/// Runs fan control until the daemon stops.
pub async fn run(shared: Arc<Shared>) {
    let mut controller = Controller::default();
    let mut helper: Option<Helper> = None;
    let mut retry_at = Instant::now();
    let mut problem = String::new();
    // Fans handed to the helper, to give back when they leave the settings.
    let mut taken: HashSet<String> = HashSet::new();
    let mut names: HashMap<String, String> = HashMap::new();
    let mut last_tick = Instant::now();

    loop {
        let settings = shared.fans.settings.lock().unwrap().clone();
        let interval = Duration::from_millis(settings.interval_ms.into());

        if !settings.controls_any() {
            if let Some(mut h) = helper.take() {
                // Dropping the connection gives everything back too; this is just quicker.
                let _ = h.command("release").await;
                info!("fan control off; every fan is back with the firmware");
            }
            taken.clear();
            controller = Controller::default();
            shared.fans.states.lock().unwrap().clear();
            shared.fans.changed.notified().await;
            continue;
        }

        if helper.is_none() && Instant::now() >= retry_at {
            match Helper::connect(&shared.fans.helper).await {
                Ok(mut h) => {
                    // Names for samples; the helper knows the hardware.
                    let latest = shared.history.lock().unwrap().back().cloned();
                    if let Ok(outputs) = h.list().await {
                        names = outputs
                            .iter()
                            .map(|o| (o.id.clone(), name_output(o, latest.as_ref()).name))
                            .collect();
                    }
                    info!("fan control: connected to the helper");
                    helper = Some(h);
                    taken.clear();
                    problem.clear();
                }
                Err(e) => {
                    if e != problem {
                        warn!("fan control: {e}");
                    }
                    problem = e;
                    retry_at = Instant::now() + RETRY;
                }
            }
        }

        let elapsed = last_tick.elapsed();
        last_tick = Instant::now();
        let history: Vec<Sample> = {
            let history = shared.history.lock().unwrap();
            let longest = settings
                .curves
                .iter()
                .map(|c| c.average_secs)
                .max()
                .unwrap_or(0);
            let count = (longest as usize * 1000 / shared.interval_ms.max(1) as usize).max(1);
            history.iter().rev().take(count).rev().cloned().collect()
        };
        let decisions = controller.tick(&settings, &history, shared.interval_ms, elapsed);

        let mut states = Vec::with_capacity(decisions.len());
        let mut lost = None;
        for (id, decision, input) in decisions {
            let name = settings.fans[&id]
                .label
                .clone()
                .or_else(|| names.get(&id).cloned())
                .unwrap_or_else(|| id.clone());
            let (percent, mut fan_problem) = match &decision {
                Decision::Set(percent) => (*percent, String::new()),
                Decision::Auto(why) => (0.0, why.clone()),
            };
            if let Some(h) = helper.as_mut().filter(|_| lost.is_none()) {
                let line = match decision {
                    Decision::Set(percent) => format!("set {id} {percent:.1}"),
                    Decision::Auto(_) => format!("auto {id}"),
                };
                match h.command(&line).await {
                    Ok(Ok(())) => {
                        if line.starts_with("set") {
                            taken.insert(id.clone());
                        } else {
                            taken.remove(&id);
                        }
                    }
                    Ok(Err(why)) => fan_problem = why,
                    Err(e) => lost = Some(e),
                }
            } else {
                fan_problem.clone_from(&problem);
            }
            states.push(FanState {
                id,
                name,
                percent,
                input,
                problem: fan_problem,
            });
        }

        // Fans set back to automatic in the settings.
        if let Some(h) = helper.as_mut().filter(|_| lost.is_none()) {
            let controlled: HashSet<&String> = states.iter().map(|s| &s.id).collect();
            let released: Vec<String> = taken
                .iter()
                .filter(|id| !controlled.contains(id))
                .cloned()
                .collect();
            for id in released {
                match h.command(&format!("auto {id}")).await {
                    Ok(_) => {
                        taken.remove(&id);
                    }
                    Err(e) => {
                        lost = Some(e);
                        break;
                    }
                }
            }
        }
        if let Some(e) = lost {
            warn!("fan control: {e}");
            for state in &mut states {
                state.problem.clone_from(&e);
            }
            problem = e;
            helper = None;
            taken.clear();
            retry_at = Instant::now() + RETRY;
        }
        *shared.fans.states.lock().unwrap() = states;

        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = shared.fans.changed.notified() => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plusysmon_protocol::fans::{FanControl, Mode};
    use plusysmon_protocol::{CpuSample, MemSample, SensorKind, SensorReading};

    fn sample(cpu: f32, gpu_like: Option<f64>) -> Sample {
        Sample {
            timestamp_ms: 0,
            cpu: CpuSample {
                total: 0.0,
                per_core: vec![],
                temperature: Some(cpu),
                frequency: None,
                frequency_peak: None,
                frequency_max: None,
                power: None,
            },
            mem: MemSample {
                total: 1,
                used: 0,
                available: 1,
                swap_total: 0,
                swap_used: 0,
            },
            gpus: vec![],
            sensors: gpu_like
                .map(|value| SensorReading {
                    id: "nvme0.temp1".into(),
                    device: "NVMe".into(),
                    driver: "nvme".into(),
                    label: "Composite".into(),
                    kind: SensorKind::Temperature,
                    value,
                    limit: None,
                })
                .into_iter()
                .collect(),
            fans: vec![],
        }
    }

    fn settings(curve: Curve, fan: FanControl) -> FanSettings {
        let mut settings = FanSettings {
            curves: vec![curve],
            ..Default::default()
        };
        settings.fans.insert("chip.pwm1".into(), fan);
        settings
    }

    fn cpu_curve() -> Curve {
        Curve {
            id: "cpu".into(),
            name: "CPU".into(),
            sources: vec!["cpu.temperature".into(), "sensor.nvme0.temp1".into()],
            combine: Mode::Highest,
            points: vec![[40.0, 20.0], [80.0, 100.0]],
            hysteresis: 3.0,
            step_up: 0.0,
            step_down: 0.0,
            ..Default::default()
        }
    }

    fn follows(curve: &str) -> FanControl {
        FanControl {
            mode: FanMode::Curve,
            curve: curve.into(),
            min: 0.0,
            ..Default::default()
        }
    }

    fn speed(controller: &mut Controller, settings: &FanSettings, history: &[Sample]) -> Decision {
        let mut decisions = controller.tick(settings, history, 1000, Duration::from_secs(3));
        assert_eq!(decisions.len(), 1);
        decisions.remove(0).1
    }

    #[test]
    fn follows_the_hottest_source_with_hysteresis() {
        let settings = settings(cpu_curve(), follows("cpu"));
        let mut c = Controller::default();
        // The NVMe drive is the hotter one: 60 °C → 60 %.
        assert_eq!(
            speed(&mut c, &settings, &[sample(50.0, Some(60.0))]),
            Decision::Set(60.0)
        );
        // Up at once.
        assert_eq!(
            speed(&mut c, &settings, &[sample(70.0, Some(60.0))]),
            Decision::Set(80.0)
        );
        // A small dip doesn't bring it down...
        assert_eq!(
            speed(&mut c, &settings, &[sample(68.0, None)]),
            Decision::Set(80.0)
        );
        // ...a real one does.
        assert_eq!(
            speed(&mut c, &settings, &[sample(66.0, None)]),
            Decision::Set(72.0)
        );
    }

    #[test]
    fn steps_down_slowly_and_respects_limits() {
        let curve = Curve {
            step_down: 2.0,
            ..cpu_curve()
        };
        let fan = FanControl {
            min: 30.0,
            max: 90.0,
            ..follows("cpu")
        };
        let settings = settings(curve, fan);
        let mut c = Controller::default();
        assert_eq!(
            speed(&mut c, &settings, &[sample(90.0, None)]),
            Decision::Set(90.0)
        );
        // Wants the minimum, gets 2 %/s over 3 s.
        assert_eq!(
            speed(&mut c, &settings, &[sample(20.0, None)]),
            Decision::Set(84.0)
        );
        assert_eq!(
            speed(&mut c, &settings, &[sample(20.0, None)]),
            Decision::Set(78.0)
        );
    }

    #[test]
    fn averages_over_the_window() {
        let curve = Curve {
            average_secs: 4,
            ..cpu_curve()
        };
        let settings = settings(curve, follows("cpu"));
        let history: Vec<Sample> = [40.0, 40.0, 80.0, 80.0, 80.0, 80.0]
            .iter()
            .map(|&t| sample(t, None))
            .collect();
        // The last four: all 80 °C.
        assert_eq!(
            speed(&mut Controller::default(), &settings, &history),
            Decision::Set(100.0)
        );
        let history: Vec<Sample> = [40.0, 40.0, 80.0, 80.0]
            .iter()
            .map(|&t| sample(t, None))
            .collect();
        assert_eq!(
            speed(&mut Controller::default(), &settings, &history),
            Decision::Set(60.0)
        );
    }

    #[test]
    fn gives_fans_back_when_the_input_goes_away() {
        let mut curve = cpu_curve();
        curve.sources = vec!["sensor.nvme0.temp1".into()];
        let settings = settings(curve, follows("cpu"));
        let mut c = Controller::default();
        assert_eq!(
            speed(&mut c, &settings, &[sample(0.0, Some(60.0))]),
            Decision::Set(60.0)
        );
        // Held through a blip...
        assert_eq!(
            speed(&mut c, &settings, &[sample(0.0, None)]),
            Decision::Set(60.0)
        );
        assert_eq!(
            speed(&mut c, &settings, &[sample(0.0, None)]),
            Decision::Set(60.0)
        );
        // ...then back to the firmware.
        assert!(matches!(
            speed(&mut c, &settings, &[sample(0.0, None)]),
            Decision::Auto(_)
        ));
        // And back under control once it returns.
        assert_eq!(
            speed(&mut c, &settings, &[sample(0.0, Some(40.0))]),
            Decision::Set(20.0)
        );
    }

    #[test]
    fn fixed_fans_need_no_input() {
        let fan = FanControl {
            mode: FanMode::Fixed,
            percent: 35.0,
            ..Default::default()
        };
        let settings = settings(cpu_curve(), fan);
        assert_eq!(
            speed(&mut Controller::default(), &settings, &[]),
            Decision::Set(35.0)
        );
    }

    #[test]
    fn names_outputs() {
        let raw = |line: &str| RawOutput::parse(line).unwrap();
        let mut s = sample(50.0, None);
        s.sensors.push(SensorReading {
            id: "it87.2624.fan2".into(),
            device: "Motherboard".into(),
            driver: "it8689".into(),
            label: "CPU fan".into(),
            kind: SensorKind::Fan,
            value: 900.0,
            limit: None,
        });
        let fan = name_output(
            &raw("out\tit87.2624.pwm2\tit8689_80060607\t40.0\t0\tit87.2624.fan2"),
            Some(&s),
        );
        assert_eq!(fan.name, "Motherboard CPU fan");
        assert_eq!(fan.rpm_source.as_deref(), Some("sensor.it87.2624.fan2"));
        assert_eq!(fan.percent, Some(40.0));

        let silent = name_output(
            &raw("out\tit87.2624.pwm4\tit8689_80060607\t-\t0\tit87.2624.fan4"),
            Some(&s),
        );
        assert_eq!(silent.name, "Motherboard Fan 4");
        assert_eq!(silent.rpm_source, None);

        let gpu = name_output(
            &raw("out\tnvidia.GPU-7b8b.fan1\tNVIDIA GeForce RTX 5070 Ti\t46.0\t30\t-"),
            None,
        );
        assert_eq!(gpu.name, "RTX 5070 Ti Fan 2");
        assert_eq!(gpu.min_percent, 30.0);
    }
}
