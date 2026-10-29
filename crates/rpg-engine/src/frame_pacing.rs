//! How often the game updates and renders, chosen to keep CPU use down where the GPU cannot help.
//!
//! On a software renderer (llvmpipe and friends) every frame is rasterized on the CPU, at roughly
//! 20 ms of CPU work per frame for the 1280x766 canvas -- over 1.5 cores at 60 Hz. Two policies:
//!
//! - Unfocused, the game wakes about once a second instead of 60 times. A key press or the window
//!   regaining focus still wakes it at once. Gameplay reads clamped frame deltas, so the world
//!   creeps rather than jumps while nobody is looking.
//! - Focused on a software adapter, the game runs a steady 30 Hz loop. It deliberately does not
//!   wake on input: holding a direction repeats key events, which would pull walking back to
//!   60 Hz. Input is still read every frame, so latency grows by at most one 33 ms frame.
//!
//! Movement and enemy deltas are clamped at 0.1 s, well above a 30 Hz frame, so the cap changes
//! smoothness, not speed.

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
            unfocused_mode: UpdateMode::reactive_low_power(UNFOCUSED_WAIT),
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

fn cap_software_renderer(
    adapter: Option<Res<RenderAdapterInfo>>,
    mut settings: ResMut<WinitSettings>,
) {
    let Some(adapter) = adapter else {
        return;
    };
    settings.focused_mode = focused_mode(adapter.device_type);
    if settings.focused_mode != UpdateMode::Continuous {
        info!(
            "Software renderer `{}`: capping the frame rate at 30 Hz to save CPU",
            adapter.name
        );
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
    fn software_frame_stays_inside_the_gameplay_delta_clamp() {
        assert!(SOFTWARE_FRAME.as_secs_f32() < 0.1);
    }
}
