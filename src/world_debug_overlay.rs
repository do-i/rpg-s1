//! Optional debug rendering of portal and collision geometry.
//!
//! Mirrors the source's own debug overlay: `engine/world/world_map_renderer.py::_render_portal_debug`
//! draws every portal as a 2px outline in `(0, 200, 255)` (a zero-size point portal draws as a 4x4
//! box), and `engine/world/player.py` (~line 266) draws the player's live collision rect as a 2px
//! outline in `(255, 0, 0)`. Both are gated in the source by an optional `debug.collision` flag in
//! `engine/settings/engine_config_data.py`, absent (off) by default.
//!
//! The port has no settings file wired up yet for this, so it follows the repo's existing
//! env-var-flag precedent instead (`RPG_S1_MUTE_AUDIO` in `src/main.rs`, `RPG_S1_SAVE_DIR` in
//! `src/save_store.rs`): set `RPG_S1_DEBUG_COLLISION` (any value) before launching to turn the
//! overlay on. It is read once at startup into [`DebugCollisionOverlay`], a plain resource, so the
//! flag can't change gameplay mid-session and every other system is unaware of it. As a bonus over
//! the source, NPC collision rects are also drawn (in magenta) when the flag is set, since the
//! source only documents portals and the player.
//!
//! Purely visual: drawn with Bevy's immediate-mode gizmos, which in 2D always render in front of
//! sprites regardless of z, so no interaction with the map's `world_entity_y_z` y-sort convention
//! is needed. The pixel-to-world axis convention matches every other World renderer (`x` unchanged,
//! `y` negated) — see `world_player::WorldPlayerMotion::sprite_center_world`.

use std::ffi::OsString;

use bevy::prelude::*;

use crate::{
    app_state::AppState,
    tmx_ground_asset::StaticMapRenderState,
    tmx_ground_asset::TmxGroundAsset,
    world_actor::WorldNpc,
    world_player::{WorldPlayer, WorldPlayerMotion},
    world_transition::runtime_portals,
};

/// Set (to any value) before launching to draw portal and collision debug outlines in the World.
pub(crate) const DEBUG_COLLISION_ENV_VAR: &str = "RPG_S1_DEBUG_COLLISION";

const PORTAL_COLOR: Color = Color::srgb(0.0, 200.0 / 255.0, 1.0);
const PLAYER_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);
const NPC_COLOR: Color = Color::srgb(1.0, 0.0, 1.0);
/// `w = portal.width if portal.width > 0 else 4` in `_render_portal_debug`.
const POINT_PORTAL_SIZE: f64 = 4.0;

/// Whether the collision/portal debug overlay is enabled for this process. Read once at startup;
/// never mutated afterward, so it cannot affect gameplay, saves, or collision resolution.
#[derive(Clone, Copy, Debug, Default, Resource)]
pub(crate) struct DebugCollisionOverlay(bool);

impl DebugCollisionOverlay {
    /// Resolves the flag from an environment lookup, injectable for tests the same way
    /// [`crate::save_store::resolve_save_directory`] takes its `environment` closure.
    pub(crate) fn from_environment(environment: impl Fn(&str) -> Option<OsString>) -> Self {
        Self(environment(DEBUG_COLLISION_ENV_VAR).is_some())
    }

    #[cfg(test)]
    pub(crate) const fn enabled(self) -> bool {
        self.0
    }
}

pub(crate) struct WorldDebugOverlayPlugin;

impl Plugin for WorldDebugOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugCollisionOverlay::from_environment(|name| {
            std::env::var_os(name)
        }))
        .add_systems(
            Update,
            draw_debug_overlay
                .run_if(|overlay: Res<DebugCollisionOverlay>| overlay.0)
                .run_if(in_state(AppState::World)),
        );
    }
}

fn draw_debug_overlay(
    maps: Res<Assets<TmxGroundAsset>>,
    render: Res<StaticMapRenderState>,
    players: Query<&WorldPlayerMotion, With<WorldPlayer>>,
    npcs: Query<&WorldNpc>,
    mut gizmos: Gizmos,
) {
    if let Some(map) = render.map(&maps)
        && let Ok(portals) = runtime_portals(map.document())
    {
        for portal in &portals {
            let bounds = portal.bounds();
            let (width, height) = effective_portal_size(bounds.width, bounds.height);
            draw_pixel_rect(&mut gizmos, bounds.x, bounds.y, width, height, PORTAL_COLOR);
        }
    }

    for motion in &players {
        let rect = motion.collision_rect();
        draw_pixel_rect(
            &mut gizmos,
            f64::from(rect.x),
            f64::from(rect.y),
            f64::from(rect.width),
            f64::from(rect.height),
            PLAYER_COLOR,
        );
    }

    for npc in &npcs {
        let rect = npc.collision_rect();
        draw_pixel_rect(
            &mut gizmos,
            f64::from(rect.x),
            f64::from(rect.y),
            f64::from(rect.width),
            f64::from(rect.height),
            NPC_COLOR,
        );
    }
}

/// `_render_portal_debug`'s `w = portal.width if portal.width > 0 else 4` (and the same for
/// height): a zero-size Tiled point object still shows as a visible 4x4 box.
fn effective_portal_size(width: f64, height: f64) -> (f64, f64) {
    let effective = |value: f64| {
        if value > 0.0 {
            value
        } else {
            POINT_PORTAL_SIZE
        }
    };
    (effective(width), effective(height))
}

/// Converts a top-left source-pixel rectangle to the Bevy-world center/size `Gizmos::rect_2d`
/// expects, using the same `x` unchanged / `y` negated axis convention as every other World
/// renderer.
fn pixel_rect_to_world(x: f64, y: f64, width: f64, height: f64) -> (Vec2, Vec2) {
    let size = Vec2::new(width as f32, height as f32);
    let center = Vec2::new(x as f32 + size.x / 2.0, -(y as f32) - size.y / 2.0);
    (center, size)
}

fn draw_pixel_rect(gizmos: &mut Gizmos, x: f64, y: f64, width: f64, height: f64, color: Color) {
    let (center, size) = pixel_rect_to_world(x, y, width, height);
    gizmos.rect_2d(center, size, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_is_off_by_default_and_on_only_when_the_env_var_is_set() {
        assert!(!DebugCollisionOverlay::from_environment(|_| None).enabled());
        assert!(
            DebugCollisionOverlay::from_environment(
                |name| (name == DEBUG_COLLISION_ENV_VAR).then(|| OsString::from("1"))
            )
            .enabled()
        );
        // Any value at all turns it on, matching `RPG_S1_MUTE_AUDIO`'s `is_some()` check.
        assert!(
            DebugCollisionOverlay::from_environment(
                |name| (name == DEBUG_COLLISION_ENV_VAR).then(|| OsString::from(""))
            )
            .enabled()
        );
        // A different variable being set must not turn the overlay on.
        assert!(
            !DebugCollisionOverlay::from_environment(
                |name| (name == "RPG_S1_MUTE_AUDIO").then(|| OsString::from("1"))
            )
            .enabled()
        );
    }

    #[test]
    fn point_portals_render_as_a_four_pixel_box_like_the_source() {
        assert_eq!(effective_portal_size(0.0, 0.0), (4.0, 4.0));
        assert_eq!(effective_portal_size(53.9015, 0.0), (53.9015, 4.0));
        assert_eq!(effective_portal_size(53.9015, 9.59847), (53.9015, 9.59847));
    }

    #[test]
    fn pixel_rect_center_negates_y_like_every_other_world_renderer() {
        let (center, size) = pixel_rect_to_world(69.4546, 124.818, 53.9015, 9.59847);
        assert_eq!(size, Vec2::new(53.9015, 9.59847));
        assert_eq!(
            center,
            Vec2::new(69.4546 + 53.9015 / 2.0, -(124.818 + 9.59847 / 2.0))
        );
    }
}
