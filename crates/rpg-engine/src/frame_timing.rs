//! Opt-in wall-clock timing for selected gameplay system hotspots, and the measurement mode that
//! turns them into a reproducible per-workload report.
//!
//! `--timings` keeps its periodic average/maximum log. `--perf-script`/`--perf-report` add
//! [`PerfRunPlugin`]: a scripted run with a fixed-length measurement window whose report carries
//! median, p95, p99 and worst frame cost, process CPU time and peak resident memory. Neither adds
//! any system or per-frame work when its flag is absent.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use bevy::{
    app::MainScheduleOrder,
    ecs::schedule::ScheduleLabel,
    prelude::*,
    render::{Render, RenderApp, RenderSystems, renderer::RenderAdapterInfo},
    winit::{UpdateMode, WinitSettings},
};

use crate::input_record::{ScriptTraceEntry, ScriptedInput};

const REPORT_EVERY_FRAMES: u64 = 120;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TimingStats {
    pub(crate) calls: u64,
    pub(crate) total: Duration,
    pub(crate) maximum: Duration,
}

impl TimingStats {
    pub(crate) fn average(self) -> Duration {
        if self.calls == 0 {
            Duration::ZERO
        } else {
            self.total / self.calls as u32
        }
    }
}

#[derive(Debug, Default, Resource)]
pub(crate) struct FrameTimings {
    samples: BTreeMap<&'static str, TimingStats>,
    frames: u64,
    /// Every individual call duration, kept only by the measurement mode for its percentiles.
    history: Option<BTreeMap<&'static str, Vec<Duration>>>,
}

impl FrameTimings {
    /// Timings that also keep each call, for a measurement run's percentiles.
    pub(crate) fn recording() -> Self {
        Self {
            history: Some(BTreeMap::new()),
            ..default()
        }
    }

    pub(crate) fn measure(&mut self, system: &'static str) -> TimingMeasurement<'_> {
        TimingMeasurement {
            started: Instant::now(),
            stats: self.samples.entry(system).or_default(),
            history: self
                .history
                .as_mut()
                .map(|history| history.entry(system).or_default()),
        }
    }

    #[cfg(test)]
    fn sample(&self, system: &str) -> Option<TimingStats> {
        self.samples.get(system).copied()
    }
}

pub(crate) struct TimingMeasurement<'a> {
    started: Instant,
    stats: &'a mut TimingStats,
    history: Option<&'a mut Vec<Duration>>,
}

impl Drop for TimingMeasurement<'_> {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed();
        self.stats.calls += 1;
        self.stats.total += elapsed;
        self.stats.maximum = self.stats.maximum.max(elapsed);
        if let Some(history) = self.history.as_mut() {
            history.push(elapsed);
        }
    }
}

pub(crate) struct FrameTimingPlugin;

impl Plugin for FrameTimingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameTimings>()
            .add_systems(Last, report_frame_timings);
    }
}

fn report_frame_timings(mut timings: ResMut<FrameTimings>) {
    timings.frames += 1;
    if timings.frames < REPORT_EVERY_FRAMES {
        return;
    }
    for (system, stats) in &timings.samples {
        println!(
            "Timing {system}: calls={} avg_us={} max_us={}",
            stats.calls,
            stats.average().as_micros(),
            stats.maximum.as_micros()
        );
    }
    timings.frames = 0;
    timings.samples.clear();
}

/// Order statistics of one measured quantity.
///
/// Percentiles use the nearest-rank method: the p-th percentile of `n` sorted samples is the
/// sample at 1-based rank `ceil(p * n / 100)`, so every reported value is one that was observed.
/// The median is the 50th percentile by the same rule (the lower middle for an even count).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Distribution {
    pub(crate) count: usize,
    pub(crate) median: Duration,
    pub(crate) p95: Duration,
    pub(crate) p99: Duration,
    pub(crate) worst: Duration,
    pub(crate) mean: Duration,
}

impl Distribution {
    pub(crate) fn from_samples(samples: &[Duration]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        let total: Duration = sorted.iter().sum();
        Some(Self {
            count: sorted.len(),
            median: nearest_rank(&sorted, 50),
            p95: nearest_rank(&sorted, 95),
            p99: nearest_rank(&sorted, 99),
            worst: sorted[sorted.len() - 1],
            mean: total / sorted.len() as u32,
        })
    }

    fn format(&self, unit: TimeUnit) -> String {
        format!(
            "median={} p95={} p99={} worst={} mean={} n={}",
            unit.format(self.median),
            unit.format(self.p95),
            unit.format(self.p99),
            unit.format(self.worst),
            unit.format(self.mean),
            self.count
        )
    }
}

/// The sample at nearest rank `ceil(percent * n / 100)` of an ascending, nonempty slice.
fn nearest_rank(sorted: &[Duration], percent: usize) -> Duration {
    let rank = (percent * sorted.len()).div_ceil(100).max(1);
    sorted[rank - 1]
}

#[derive(Clone, Copy)]
enum TimeUnit {
    Milliseconds,
    Microseconds,
}

impl TimeUnit {
    fn format(self, value: Duration) -> String {
        match self {
            Self::Milliseconds => format!("{:.3}", value.as_secs_f64() * 1_000.0),
            Self::Microseconds => format!("{:.1}", value.as_secs_f64() * 1_000_000.0),
        }
    }
}

/// Runs just before [`First`] in measurement mode: the start of a main-schedule frame.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct PerfFrameStart;

/// Runs just after [`Last`] in measurement mode: the end of a main-schedule frame.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct PerfFrameEnd;

/// Render-world schedule spans, shared with the render thread under pipelined rendering.
#[derive(Clone, Default, Resource)]
struct RenderSpanLog(Arc<Mutex<RenderSpans>>);

#[derive(Default)]
struct RenderSpans {
    recording: bool,
    started: Option<Instant>,
    samples: Vec<Duration>,
}

impl RenderSpanLog {
    fn with<R>(&self, action: impl FnOnce(&mut RenderSpans) -> R) -> R {
        action(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

struct MeasureWindow {
    frames: u64,
    first_frame: u64,
    opened: Instant,
    cpu_ticks: Option<u64>,
    main: Vec<Duration>,
    interval: Vec<Duration>,
    entities: Vec<u32>,
}

/// One measurement run's configuration and state. Its presence is what selects measurement mode.
#[derive(Resource)]
pub(crate) struct PerfRun {
    report: PathBuf,
    seed: u64,
    launched: Instant,
    frame: u64,
    frame_started: Option<Instant>,
    previous_start: Option<Instant>,
    interval: Option<Duration>,
    first_frame: Option<Duration>,
    window: Option<MeasureWindow>,
    finished: bool,
}

impl PerfRun {
    /// `launched` is when the game launcher started, so `first_frame_ms` covers app construction.
    pub(crate) fn new(report: PathBuf, seed: u64, launched: Instant) -> Self {
        Self {
            report,
            seed,
            launched,
            frame: 0,
            frame_started: None,
            previous_start: None,
            interval: None,
            first_frame: None,
            window: None,
            finished: false,
        }
    }
}

/// Measurement mode. Added only when `--perf-report` is given.
pub(crate) struct PerfRunPlugin;

impl Plugin for PerfRunPlugin {
    fn build(&self, app: &mut App) {
        let log = RenderSpanLog::default();
        app.insert_resource(FrameTimings::recording())
            .insert_resource(log.clone())
            .init_schedule(PerfFrameStart)
            .init_schedule(PerfFrameEnd)
            .add_systems(
                PreUpdate,
                crate::input_record::apply_scripted_actions
                    .after(crate::action_input::update_action_state),
            )
            .add_systems(PerfFrameStart, start_perf_frame)
            .add_systems(PerfFrameEnd, finish_perf_frame);
        let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
        order.insert_before(First, PerfFrameStart);
        order.insert_after(Last, PerfFrameEnd);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.insert_resource(log).add_systems(
                Render,
                (
                    start_render_span.before(RenderSystems::ExtractCommands),
                    finish_render_span.after(RenderSystems::PostCleanup),
                ),
            );
        }
    }
}

fn start_render_span(log: Res<RenderSpanLog>) {
    log.with(|spans| spans.started = Some(Instant::now()));
}

fn finish_render_span(log: Res<RenderSpanLog>) {
    log.with(|spans| {
        if let Some(started) = spans.started.take()
            && spans.recording
        {
            spans.samples.push(started.elapsed());
        }
    });
}

fn start_perf_frame(mut perf: ResMut<PerfRun>) {
    let now = Instant::now();
    perf.frame += 1;
    perf.interval = perf.previous_start.map(|previous| now - previous);
    perf.previous_start = Some(now);
    perf.frame_started = Some(now);
    if perf.first_frame.is_none() {
        perf.first_frame = Some(now - perf.launched);
    }
}

fn finish_perf_frame(world: &mut World) {
    let ended = Instant::now();
    if world.resource::<PerfRun>().finished {
        return;
    }
    let entities = world.entity_count();
    let (request, failure) = world
        .get_resource_mut::<ScriptedInput>()
        .map(|mut script| (script.measure_request.take(), script.failure.clone()))
        .unwrap_or_default();

    let mut perf = world.resource_mut::<PerfRun>();
    let main = perf
        .frame_started
        .map_or(Duration::ZERO, |started| ended - started);
    let interval = perf.interval;
    let frame = perf.frame;
    let complete = match perf.window.as_mut() {
        Some(window) => {
            window.main.push(main);
            if let Some(interval) = interval {
                window.interval.push(interval);
            }
            window.entities.push(entities);
            window.main.len() as u64 >= window.frames
        }
        None => {
            if let Some(frames) = request {
                perf.window = Some(MeasureWindow {
                    frames,
                    first_frame: frame + 1,
                    opened: ended,
                    cpu_ticks: process_cpu_ticks(),
                    main: Vec::with_capacity(frames as usize),
                    interval: Vec::with_capacity(frames as usize),
                    entities: Vec::with_capacity(frames as usize),
                });
                world
                    .resource::<RenderSpanLog>()
                    .with(|spans| spans.recording = true);
                if let Some(history) = world.resource_mut::<FrameTimings>().history.as_mut() {
                    history.clear();
                }
            }
            false
        }
    };
    if complete || failure.is_some() {
        let status = failure.map_or_else(|| "ok".to_owned(), |error| format!("FAIL: {error}"));
        write_perf_report(world, status);
    }
}

fn write_perf_report(world: &mut World, mut status: String) {
    let render_samples = world.resource::<RenderSpanLog>().with(|spans| {
        spans.recording = false;
        std::mem::take(&mut spans.samples)
    });
    let systems = world
        .resource::<FrameTimings>()
        .history
        .as_ref()
        .map(|history| {
            history
                .iter()
                .filter_map(|(system, samples)| {
                    Distribution::from_samples(samples).map(|stats| (*system, stats))
                })
                .collect()
        })
        .unwrap_or_default();
    let (trace, remaining) = world
        .get_resource::<ScriptedInput>()
        .map(|script| (script.trace.clone(), script.remaining_steps()))
        .unwrap_or_default();
    if status == "ok" && remaining > 0 {
        status = format!(
            "FAIL: the measurement window closed with {remaining} script steps still to run"
        );
    }
    let adapter = world.get_resource::<RenderAdapterInfo>().map_or_else(
        || "unknown".to_owned(),
        |adapter| {
            format!(
                "{} ({:?}, {:?}, driver {} {})",
                adapter.name,
                adapter.device_type,
                adapter.backend,
                adapter.driver,
                adapter.driver_info
            )
        },
    );
    let pacing = world.get_resource::<WinitSettings>().map_or_else(
        || "unknown".to_owned(),
        |settings| {
            format!(
                "focused {}; unfocused {}",
                describe_update_mode(settings.focused_mode),
                describe_update_mode(settings.unfocused_mode)
            )
        },
    );
    let mut perf = world.resource_mut::<PerfRun>();
    perf.finished = true;
    let window = perf.window.take();
    let cpu_seconds = window.as_ref().and_then(|window| {
        let start = window.cpu_ticks?;
        let end = process_cpu_ticks()?;
        Some(end.saturating_sub(start) as f64 / CLOCK_TICKS_PER_SECOND)
    });
    let memory = process_memory();
    let report = PerfReport {
        status,
        build: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        adapter,
        pacing,
        seed: perf.seed,
        first_frame: perf.first_frame,
        window_first_frame: window.as_ref().map_or(0, |window| window.first_frame),
        frames: window.as_ref().map_or(0, |window| window.main.len()),
        window_wall: window
            .as_ref()
            .map_or(Duration::ZERO, |window| window.opened.elapsed()),
        main: window
            .as_ref()
            .and_then(|window| Distribution::from_samples(&window.main)),
        render: Distribution::from_samples(&render_samples),
        interval: window
            .as_ref()
            .and_then(|window| Distribution::from_samples(&window.interval)),
        systems,
        cpu_seconds,
        peak_rss_kib: memory.map(|(peak, _)| peak),
        end_rss_kib: memory.map(|(_, end)| end),
        entities: window.as_ref().and_then(|window| {
            Some((
                *window.entities.iter().min()?,
                *window.entities.iter().max()?,
            ))
        }),
        trace,
    };
    let text = report.render();
    print!("{text}");
    let failed = !report.status.starts_with("ok");
    let mut written = fs::write(&perf.report, &text);
    if let Some(window) = window.as_ref() {
        let mut frames = String::from("frame,interval_us,main_us,entities\n");
        for (index, main) in window.main.iter().enumerate() {
            let interval = window
                .interval
                .get(index)
                .map_or(String::new(), |value| value.as_micros().to_string());
            let _ = writeln!(
                frames,
                "{},{},{},{}",
                window.first_frame + index as u64,
                interval,
                main.as_micros(),
                window.entities[index]
            );
        }
        written =
            written.and_then(|()| fs::write(perf.report.with_extension("frames.csv"), frames));
    }
    let mut render = String::from("span,render_us\n");
    for (index, span) in render_samples.iter().enumerate() {
        let _ = writeln!(render, "{index},{}", span.as_micros());
    }
    written = written.and_then(|()| fs::write(perf.report.with_extension("render.csv"), render));
    if let Err(error) = &written {
        eprintln!(
            "Perf report could not be written to `{}`: {error}",
            perf.report.display()
        );
    }
    world.write_message(if failed || written.is_err() {
        AppExit::error()
    } else {
        AppExit::Success
    });
}

fn describe_update_mode(mode: UpdateMode) -> String {
    match mode {
        UpdateMode::Continuous => "continuous".to_owned(),
        UpdateMode::Reactive { wait, .. } => format!(
            "reactive, wait {:.3} ms ({:.1} Hz cap)",
            wait.as_secs_f64() * 1_000.0,
            1.0 / wait.as_secs_f64()
        ),
    }
}

/// Linux reports process CPU time in `USER_HZ` ticks, which the kernel ABI fixes at 100.
const CLOCK_TICKS_PER_SECOND: f64 = 100.0;

/// User plus system CPU ticks of every thread in this process. Linux-only; `None` elsewhere.
fn process_cpu_ticks() -> Option<u64> {
    parse_cpu_ticks(&fs::read_to_string("/proc/self/stat").ok()?)
}

fn parse_cpu_ticks(stat: &str) -> Option<u64> {
    // The command name may contain spaces and parentheses, so count fields from its last `)`.
    // After it come field 3 (state) onward; utime and stime are fields 14 and 15.
    let mut fields = stat.get(stat.rfind(')')? + 1..)?.split_whitespace();
    let user: u64 = fields.nth(11)?.parse().ok()?;
    let system: u64 = fields.next()?.parse().ok()?;
    Some(user + system)
}

/// Peak and current resident set size in KiB (`VmHWM`, `VmRSS`). Linux-only; `None` elsewhere.
fn process_memory() -> Option<(u64, u64)> {
    parse_memory(&fs::read_to_string("/proc/self/status").ok()?)
}

fn parse_memory(status: &str) -> Option<(u64, u64)> {
    let field = |name: &str| -> Option<u64> {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))?
            .trim()
            .strip_suffix("kB")?
            .trim()
            .parse()
            .ok()
    };
    Some((field("VmHWM:")?, field("VmRSS:")?))
}

/// Everything one measurement run reports, rendered as `key: value` lines.
struct PerfReport {
    status: String,
    build: &'static str,
    adapter: String,
    pacing: String,
    seed: u64,
    first_frame: Option<Duration>,
    window_first_frame: u64,
    frames: usize,
    window_wall: Duration,
    main: Option<Distribution>,
    render: Option<Distribution>,
    interval: Option<Distribution>,
    systems: Vec<(&'static str, Distribution)>,
    cpu_seconds: Option<f64>,
    peak_rss_kib: Option<u64>,
    end_rss_kib: Option<u64>,
    entities: Option<(u32, u32)>,
    trace: Vec<ScriptTraceEntry>,
}

impl PerfReport {
    fn render(&self) -> String {
        let mut text = String::new();
        let mut line = |key: &str, value: String| {
            let _ = writeln!(text, "{key}: {value}");
        };
        let unavailable = || "unavailable".to_owned();
        line("status", self.status.clone());
        line("build", self.build.to_owned());
        line("adapter", self.adapter.clone());
        line("pacing", self.pacing.clone());
        line("seed", self.seed.to_string());
        line(
            "first_frame_ms",
            self.first_frame
                .map_or_else(unavailable, |value| TimeUnit::Milliseconds.format(value)),
        );
        line("window_first_frame", self.window_first_frame.to_string());
        line("frames", self.frames.to_string());
        line(
            "window_wall_s",
            format!("{:.3}", self.window_wall.as_secs_f64()),
        );
        for (key, distribution) in [
            ("main_ms", self.main),
            ("render_ms", self.render),
            ("interval_ms", self.interval),
        ] {
            line(
                key,
                distribution.map_or_else(unavailable, |stats| stats.format(TimeUnit::Milliseconds)),
            );
        }
        for (system, stats) in &self.systems {
            line(
                &format!("system_us {system}"),
                stats.format(TimeUnit::Microseconds),
            );
        }
        let wall = self.window_wall.as_secs_f64();
        line(
            "cpu_s_per_s",
            self.cpu_seconds
                .filter(|_| wall > 0.0)
                .map_or_else(unavailable, |cpu| format!("{:.3}", cpu / wall)),
        );
        line(
            "cpu_ms_per_frame",
            self.cpu_seconds
                .filter(|_| self.frames > 0)
                .map_or_else(unavailable, |cpu| {
                    format!("{:.3}", cpu * 1_000.0 / self.frames as f64)
                }),
        );
        let mebibytes = |kib: Option<u64>| {
            kib.map_or_else(unavailable, |kib| format!("{:.1}", kib as f64 / 1_024.0))
        };
        line("peak_rss_mib", mebibytes(self.peak_rss_kib));
        line("end_rss_mib", mebibytes(self.end_rss_kib));
        line(
            "entities",
            self.entities
                .map_or_else(unavailable, |(min, max)| format!("min={min} max={max}")),
        );
        for entry in &self.trace {
            line(
                "trace",
                format!(
                    "frame={} ms={} state={} map={} line={} step={}",
                    entry.frame,
                    entry.elapsed_ms,
                    entry.app_state,
                    entry.map,
                    entry.line,
                    entry.step
                ),
            );
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measurement_guard_records_each_system_without_changing_its_result() {
        let mut timings = FrameTimings::default();
        let result = {
            let _measurement = timings.measure("world.invented");
            (2_u64..=5).product::<u64>()
        };

        assert_eq!(result, 120);
        let stats = timings.sample("world.invented").unwrap();
        assert_eq!(stats.calls, 1);
        assert!(stats.total >= stats.average());
        assert!(stats.maximum <= stats.total);
        assert!(timings.history.is_none());
    }

    #[test]
    fn recording_timings_keep_every_call_for_percentiles() {
        let mut timings = FrameTimings::recording();
        for _ in 0..3 {
            let _measurement = timings.measure("world.invented");
        }

        assert_eq!(timings.sample("world.invented").unwrap().calls, 3);
        assert_eq!(timings.history.as_ref().unwrap()["world.invented"].len(), 3);
    }

    fn millis(values: impl IntoIterator<Item = u64>) -> Vec<Duration> {
        values.into_iter().map(Duration::from_millis).collect()
    }

    #[test]
    fn percentiles_use_nearest_rank_over_unsorted_samples() {
        // 1..=100 ms shuffled: nearest rank puts p50 at 50, p95 at 95 and p99 at 99.
        let mut samples = millis(1..=100);
        samples.reverse();
        samples.swap(3, 70);
        let stats = Distribution::from_samples(&samples).unwrap();

        assert_eq!(stats.count, 100);
        assert_eq!(stats.median, Duration::from_millis(50));
        assert_eq!(stats.p95, Duration::from_millis(95));
        assert_eq!(stats.p99, Duration::from_millis(99));
        assert_eq!(stats.worst, Duration::from_millis(100));
        assert_eq!(stats.mean, Duration::from_micros(50_500));
    }

    #[test]
    fn small_samples_round_ranks_up_and_one_hitch_is_the_worst() {
        let stats = Distribution::from_samples(&millis([4, 4, 4, 90])).unwrap();
        assert_eq!(stats.median, Duration::from_millis(4));
        assert_eq!(stats.p95, Duration::from_millis(90));
        assert_eq!(stats.p99, Duration::from_millis(90));
        assert_eq!(stats.worst, Duration::from_millis(90));

        let single = Distribution::from_samples(&millis([7])).unwrap();
        assert_eq!(
            (single.median, single.p99, single.worst),
            (
                Duration::from_millis(7),
                Duration::from_millis(7),
                Duration::from_millis(7)
            )
        );
        assert_eq!(Distribution::from_samples(&[]), None);
    }

    #[test]
    fn proc_parsers_read_cpu_ticks_and_resident_memory() {
        let stat = "4242 (rpg s1 (main)) S 1 4242 4242 0 -1 4194560 100 0 0 0 1500 250 0 0 20 0 9";
        assert_eq!(parse_cpu_ticks(stat), Some(1_750));
        assert_eq!(parse_cpu_ticks("garbage"), None);

        let status = "Name:\trpg-s1\nVmPeak:\t 9000 kB\nVmHWM:\t  524288 kB\nVmRSS:\t  262144 kB\n";
        assert_eq!(parse_memory(status), Some((524_288, 262_144)));
        assert_eq!(parse_memory("Name:\trpg-s1\n"), None);
    }

    #[test]
    fn report_renders_one_parseable_key_value_line_per_metric() {
        let report = PerfReport {
            status: "ok".to_owned(),
            build: "release",
            adapter: "llvmpipe (Cpu, Vulkan, driver llvmpipe Mesa)".to_owned(),
            pacing: describe_update_mode(UpdateMode::reactive(Duration::from_millis(40))),
            seed: 1,
            first_frame: Some(Duration::from_millis(1_250)),
            window_first_frame: 31,
            frames: 4,
            window_wall: Duration::from_secs(2),
            main: Distribution::from_samples(&millis([4, 4, 4, 90])),
            render: None,
            interval: Distribution::from_samples(&millis([33, 33, 34, 120])),
            systems: vec![(
                "world.npcs",
                Distribution::from_samples(&[Duration::from_micros(15)]).unwrap(),
            )],
            cpu_seconds: Some(1.5),
            peak_rss_kib: Some(524_288),
            end_rss_kib: None,
            entities: Some((10, 12)),
            trace: vec![ScriptTraceEntry {
                frame: 30,
                elapsed_ms: 1_000,
                line: 2,
                step: "wait_state World".to_owned(),
                app_state: "World".to_owned(),
                map: "town_01_ardel".to_owned(),
            }],
        };

        let text = report.render();
        let expected = "\
status: ok
build: release
adapter: llvmpipe (Cpu, Vulkan, driver llvmpipe Mesa)
pacing: reactive, wait 40.000 ms (25.0 Hz cap)
seed: 1
first_frame_ms: 1250.000
window_first_frame: 31
frames: 4
window_wall_s: 2.000
main_ms: median=4.000 p95=90.000 p99=90.000 worst=90.000 mean=25.500 n=4
render_ms: unavailable
interval_ms: median=33.000 p95=120.000 p99=120.000 worst=120.000 mean=55.000 n=4
system_us world.npcs: median=15.0 p95=15.0 p99=15.0 worst=15.0 mean=15.0 n=1
cpu_s_per_s: 0.750
cpu_ms_per_frame: 375.000
peak_rss_mib: 512.0
end_rss_mib: unavailable
entities: min=10 max=12
trace: frame=30 ms=1000 state=World map=town_01_ardel line=2 step=wait_state World
";
        assert_eq!(text, expected);
    }
}
