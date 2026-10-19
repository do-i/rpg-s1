//! Out-of-battle MP regeneration while the party walks or rests in the World.
//!
//! Rates come from `balance.yaml`'s `mp_regen` group ([`MpRegenBalance`]). A step is one tile of
//! the player's actual displacement, so walking into a wall earns nothing. A map with wandering
//! enemies ticks on steps alone; a safe map (no encounter zone: towns and interiors) ticks faster
//! on steps and also on elapsed time, so standing in town refills MP. Nothing ticks while an
//! overlay owns the screen, during a map transition, or while the player has paused, and a
//! knocked-out member never regenerates.

use bevy::prelude::*;
use rpg_content::scenario_balance::{BalanceData, MpRegenBalance};

use crate::{
    app_state::AppState, game_state::GameState, runtime_member::RuntimeMember,
    world_encounter::WorldEncounterState, world_player::WorldPlayerMotion,
};

const STEP_PIXELS: f32 = 32.0;
/// A larger single-frame displacement is a warp or scripted placement, not walking.
const MAX_WALKED_PIXELS_PER_FRAME: f32 = STEP_PIXELS * 2.0;

pub(crate) struct WorldMpRegenPlugin;

impl Plugin for WorldMpRegenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MpRegenClock>()
            .add_systems(OnEnter(AppState::World), reset_mp_regen_clock)
            .add_systems(
                Update,
                regenerate_party_mana
                    .run_if(in_state(AppState::World))
                    .run_if(crate::world_pause::world_simulation_running),
            );
    }
}

/// Partial progress toward the next tick. Not saved: at most one tick is lost across a load.
#[derive(Clone, Debug, Default, PartialEq, Resource)]
pub(crate) struct MpRegenClock {
    map_id: Option<String>,
    last_top_left: Option<Vec2>,
    walked_pixels: f32,
    rested_seconds: f32,
}

impl MpRegenClock {
    /// Advances the clock by one frame and returns how many ticks completed.
    ///
    /// `safe` is `None` while the map's encounters are still loading; the position is tracked
    /// but nothing accrues. Entering a different map discards partial progress.
    fn advance(
        &mut self,
        map_id: &str,
        top_left: Vec2,
        delta_seconds: f32,
        safe: Option<bool>,
        balance: &MpRegenBalance,
    ) -> u32 {
        if self.map_id.as_deref() != Some(map_id) {
            *self = Self {
                map_id: Some(map_id.to_owned()),
                last_top_left: Some(top_left),
                ..Self::default()
            };
            return 0;
        }
        let walked = self
            .last_top_left
            .replace(top_left)
            .map_or(0.0, |last| last.distance(top_left));
        let Some(safe) = safe else {
            return 0;
        };
        if walked <= MAX_WALKED_PIXELS_PER_FRAME {
            self.walked_pixels += walked;
        }
        let steps_per_tick = if safe {
            balance.safe_steps_per_tick.get()
        } else {
            balance.field_steps_per_tick.get()
        };
        let pixels_per_tick = STEP_PIXELS * steps_per_tick as f32;
        let mut ticks = 0;
        while self.walked_pixels >= pixels_per_tick {
            self.walked_pixels -= pixels_per_tick;
            ticks += 1;
        }
        if safe {
            let seconds_per_tick = balance.safe_seconds_per_tick.get() as f32;
            self.rested_seconds += delta_seconds;
            while self.rested_seconds >= seconds_per_tick {
                self.rested_seconds -= seconds_per_tick;
                ticks += 1;
            }
        }
        ticks
    }
}

/// Applies `ticks` to one member: each restores a percentage of max MP, at least one point.
fn regenerate(member: &mut RuntimeMember, ticks: u32, balance: &MpRegenBalance) {
    if member.is_knocked_out() {
        return;
    }
    let percent = u64::from(balance.max_percent_per_tick.get());
    let per_tick = (u64::from(member.max_mana()) * percent / 100).max(1);
    let amount = per_tick.saturating_mul(u64::from(ticks));
    member.restore_mana(u32::try_from(amount).unwrap_or(u32::MAX));
}

fn reset_mp_regen_clock(mut clock: ResMut<MpRegenClock>) {
    *clock = MpRegenClock::default();
}

fn regenerate_party_mana(
    time: Res<Time>,
    balances: Res<Assets<BalanceData>>,
    encounters: Option<Res<WorldEncounterState>>,
    players: Query<&WorldPlayerMotion>,
    game: Option<ResMut<GameState>>,
    mut clock: ResMut<MpRegenClock>,
) {
    let Some(mut game) = game else {
        return;
    };
    let Ok(motion) = players.single() else {
        return;
    };
    let Some(map_id) = game.map().current().map(|map| map.as_str().to_owned()) else {
        return;
    };
    let balance = balances.iter().next().map_or_else(
        || BalanceData::default().mp_regen,
        |(_, balance)| balance.mp_regen.clone(),
    );
    let safe = encounters
        .as_deref()
        .and_then(|state| state.ambient_encounters(&map_id))
        .map(|ambient| !ambient);
    let ticks = clock.advance(
        &map_id,
        motion.top_left(),
        time.delta_secs(),
        safe,
        &balance,
    );
    if ticks == 0 {
        return;
    }
    for member in game.party_mut().members_mut() {
        regenerate(member, ticks, &balance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exactly representable, so ten seconds of frames sum to exactly ten.
    const FRAME: f32 = 1.0 / 64.0;

    fn walk(clock: &mut MpRegenClock, map: &str, from: Vec2, tiles: u32, safe: bool) -> u32 {
        let balance = MpRegenBalance::default();
        let mut ticks = 0;
        let mut at = from;
        // Five pixels a frame, the player's walking speed.
        for _ in 0..(tiles * 32 / 5 + 1) {
            at.x += 5.0;
            ticks += clock.advance(map, at, 0.0, Some(safe), &balance);
        }
        ticks
    }

    fn started(map: &str) -> MpRegenClock {
        let mut clock = MpRegenClock::default();
        clock.advance(
            map,
            Vec2::ZERO,
            0.0,
            Some(false),
            &MpRegenBalance::default(),
        );
        clock
    }

    #[test]
    fn field_steps_tick_every_eight_tiles_and_safe_steps_every_four() {
        assert_eq!(walk(&mut started("zone"), "zone", Vec2::ZERO, 16, false), 2);
        assert_eq!(walk(&mut started("town"), "town", Vec2::ZERO, 16, true), 4);
    }

    #[test]
    fn standing_still_ticks_only_on_a_safe_map() {
        let balance = MpRegenBalance::default();
        for (safe, expected) in [(false, 0), (true, 5)] {
            let mut clock = started("map");
            let ticks: u32 = (0..640)
                .map(|_| clock.advance("map", Vec2::ZERO, FRAME, Some(safe), &balance))
                .sum();
            assert_eq!(ticks, expected, "safe = {safe}");
        }
    }

    #[test]
    fn unresolved_encounters_accrue_nothing_but_keep_tracking_position() {
        let balance = MpRegenBalance::default();
        let mut clock = started("map");
        assert_eq!(
            clock.advance("map", Vec2::new(500.0, 0.0), 30.0, None, &balance),
            0
        );
        assert_eq!(clock.walked_pixels, 0.0);
        assert_eq!(clock.rested_seconds, 0.0);
        assert_eq!(clock.last_top_left, Some(Vec2::new(500.0, 0.0)));
    }

    #[test]
    fn warps_and_map_changes_earn_no_steps() {
        let balance = MpRegenBalance::default();
        let mut clock = started("zone");
        clock.advance("zone", Vec2::new(100.0, 0.0), 0.0, Some(false), &balance);
        assert_eq!(clock.walked_pixels, 0.0, "a 100px jump is a warp");

        clock.advance("zone", Vec2::new(130.0, 0.0), 0.0, Some(false), &balance);
        assert_eq!(clock.walked_pixels, 30.0);
        clock.advance("cave", Vec2::new(900.0, 0.0), 0.0, Some(false), &balance);
        assert_eq!(
            clock.walked_pixels, 0.0,
            "a new map discards partial progress"
        );
    }

    fn member(health: u32, mana: u32, max_mana: u32) -> RuntimeMember {
        let parts = crate::runtime_member::RuntimeMemberParts {
            id: "mage".to_owned(),
            name: "Mage".to_owned(),
            protagonist: false,
            class_id: "mage".to_owned(),
            level: 1,
            experience: 0,
            experience_next: 10,
            health,
            max_health: 10,
            mana,
            max_mana,
            stats: [1, 1, 1, 1],
            row: crate::scenario_party::PartyRow::Back,
            tactics: crate::battle_tactics::BattleTactics::default(),
            equipment: Default::default(),
            status_effects: Vec::new(),
        };
        RuntimeMember::try_from_saved(parts, &BalanceData::default().progression)
            .expect("valid member")
    }

    #[test]
    fn a_tick_restores_one_percent_of_max_but_at_least_one_point() {
        let balance = MpRegenBalance::default();
        let mut small = member(10, 0, 20);
        regenerate(&mut small, 3, &balance);
        assert_eq!(small.mana(), 3);

        let mut large = member(10, 0, 640);
        regenerate(&mut large, 2, &balance);
        assert_eq!(large.mana(), 12);

        let mut nearly_full = member(10, 19, 20);
        regenerate(&mut nearly_full, 5, &balance);
        assert_eq!(nearly_full.mana(), 20, "never past max");
    }

    #[test]
    fn a_knocked_out_member_does_not_regenerate() {
        let mut fallen = member(0, 0, 20);
        regenerate(&mut fallen, 10, &MpRegenBalance::default());
        assert_eq!(fallen.mana(), 0);
    }
}
