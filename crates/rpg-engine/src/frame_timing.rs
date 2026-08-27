//! Opt-in wall-clock timing for selected gameplay system hotspots.

use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use bevy::prelude::*;

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
}

impl FrameTimings {
    pub(crate) fn measure(&mut self, system: &'static str) -> TimingMeasurement<'_> {
        TimingMeasurement {
            started: Instant::now(),
            stats: self.samples.entry(system).or_default(),
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
}

impl Drop for TimingMeasurement<'_> {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed();
        self.stats.calls += 1;
        self.stats.total += elapsed;
        self.stats.maximum = self.stats.maximum.max(elapsed);
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
    }
}
