//! How often the game updates and renders, chosen to keep CPU use down where the GPU cannot help.
//!
//! On a software renderer (llvmpipe and friends) every frame is rasterized on the CPU: about 20 ms
//! of work per frame at the 1280x766 canvas, which is over 1.5 cores at 60 Hz. Two policies:
//!
//! - Unfocused, the game wakes about once a second. A key press or regaining focus wakes it at
//!   once. Gameplay reads clamped frame deltas, so the world creeps rather than jumps.
//! - Focused on a software adapter, the game runs a steady 30 Hz loop. It deliberately does not
//!   wake on input: holding a direction repeats key events, which would pull walking back to
//!   60 Hz. Input is still read every frame, so latency grows by at most one 33 ms frame.
//!
//! Movement and enemy deltas are clamped at 0.1 s, well above a 30 Hz frame, so the cap changes
//! smoothness, not speed.
//!
//! A measurement run (`--perf-report`) keeps the software cap, since that is how the game ships on
//! such a machine, but runs unfocused at the focused rate: under a virtual display with no window
//! manager the window never gains focus, and a 1 Hz run measures nothing useful.

use std::time::Duration;

use bevy::{
    prelude::*,
    render::renderer::RenderAdapterInfo,
    winit::{UpdateMode, WinitSettings},
};
use wgpu_types::DeviceType;

const UNFOCUSED_WAIT: Duration = Duration::from_secs(1);
const SOFTWARE_FRAME: Duration = Duration::from_nanos(1_000_000_000 / 30);

pub(crate) struct FramePacingPlugin;

impl Plugin for FramePacingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: unfocused_mode(UpdateMode::Continuous, false),
        })
        .add_systems(Startup, cap_software_renderer);
    }
}

/// The focused update mode for an adapter of `device_type`.
fn focused_mode(device_type: DeviceType) -> UpdateMode {
    if device_type == DeviceType::Cpu {
        UpdateMode::Reactive {
            wait: SOFTWARE_FRAME,
            react_to_device_events: false,
            react_to_user_events: false,
            react_to_window_events: false,
        }
    } else {
        UpdateMode::Continuous
    }
}

/// The unfocused update mode: the 1 Hz power saver, or the focused mode in a measurement run.
fn unfocused_mode(focused: UpdateMode, measuring: bool) -> UpdateMode {
    if measuring {
        focused
    } else {
        UpdateMode::reactive_low_power(UNFOCUSED_WAIT)
    }
}

fn cap_software_renderer(
    adapter: Option<Res<RenderAdapterInfo>>,
    measurement: Option<Res<crate::frame_timing::PerfRun>>,
    mut settings: ResMut<WinitSettings>,
) {
    if let Some(adapter) = adapter {
        settings.focused_mode = focused_mode(adapter.device_type);
        if settings.focused_mode != UpdateMode::Continuous {
            info!(
                "Software renderer `{}`: capping the frame rate at 30 Hz to save CPU",
                adapter.name
            );
        }
    }
    if measurement.is_some() {
        settings.unfocused_mode = unfocused_mode(settings.focused_mode, true);
        info!("Measurement run: the unfocused window updates at the focused rate");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_software_adapter_is_capped() {
        for device_type in [
            DeviceType::DiscreteGpu,
            DeviceType::IntegratedGpu,
            DeviceType::VirtualGpu,
            DeviceType::Other,
        ] {
            assert_eq!(focused_mode(device_type), UpdateMode::Continuous);
        }
        let UpdateMode::Reactive {
            wait,
            react_to_device_events,
            react_to_user_events,
            react_to_window_events,
        } = focused_mode(DeviceType::Cpu)
        else {
            panic!("a software adapter should run a reactive loop");
        };
        assert_eq!(wait, SOFTWARE_FRAME);
        assert!(!react_to_device_events && !react_to_user_events && !react_to_window_events);
    }

    #[test]
    fn only_a_measurement_run_lifts_the_unfocused_throttle_and_it_keeps_the_cap() {
        let capped = focused_mode(DeviceType::Cpu);
        assert_eq!(
            unfocused_mode(capped, false),
            UpdateMode::reactive_low_power(UNFOCUSED_WAIT)
        );
        assert_eq!(
            unfocused_mode(UpdateMode::Continuous, false),
            UpdateMode::reactive_low_power(UNFOCUSED_WAIT)
        );
        assert_eq!(unfocused_mode(capped, true), capped);
    }

    #[test]
    fn software_frame_stays_inside_the_gameplay_delta_clamp() {
        assert!(SOFTWARE_FRAME.as_secs_f32() < 0.1);
    }
}
