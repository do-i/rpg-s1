//! The optional v2 enemy battler sheet: one pose per status, in a 4x4 grid of 256px tiles.
//!
//! The sheet lives beside the v1 battler at `media/sprites/enemies/v2/{id}-status.tsx`. An enemy
//! without one keeps drawing from its v1 sheet exactly as before, so migration is per enemy.
//! The tile table is documented in `docs/enemy-battler-tiles.md`.

use crate::scenario_path::{ScenarioRelativePath, ScenarioRelativePathError};

use super::{
    fx::AttackKind,
    status::{ActiveStatus, StatusEffect},
};

pub(super) const READY_TILE: u32 = 0;
pub(super) const KNOCKED_OUT_TILE: u32 = 14;

pub(super) fn status_sheet_path(
    sprite_id: &str,
) -> Result<ScenarioRelativePath, ScenarioRelativePathError> {
    ScenarioRelativePath::try_from(
        format!("media/sprites/enemies/v2/{sprite_id}-status.tsx").as_str(),
    )
}

/// The optional attack sheet beside the status sheet: two tiles in one row.
pub(super) fn attack_sheet_path(
    sprite_id: &str,
) -> Result<ScenarioRelativePath, ScenarioRelativePathError> {
    ScenarioRelativePath::try_from(
        format!("media/sprites/enemies/v2/{sprite_id}-attack.tsx").as_str(),
    )
}

/// The attack-sheet tile for one action: a basic attack thrusts, an ability lunges.
pub(super) const fn attack_tile(kind: AttackKind) -> u32 {
    match kind {
        AttackKind::Thrust => 0,
        AttackKind::Spellcast => 1,
    }
}

/// Statuses that own a pose, strongest first. The first one an enemy carries wins.
const PRECEDENCE: [(StatusEffect, u32); 13] = [
    (StatusEffect::Freeze, 6),
    (StatusEffect::Sleep, 2),
    (StatusEffect::Stun, 3),
    (StatusEffect::Burn, 5),
    (StatusEffect::Poison, 1),
    (StatusEffect::Silence, 4),
    (StatusEffect::Knockback, 7),
    (StatusEffect::RedirectDamage, 10),
    (StatusEffect::Taunt, 8),
    (StatusEffect::DamageReduction, 9),
    (StatusEffect::Slow, 12),
    (StatusEffect::Haste, 11),
    (StatusEffect::Regen, 13),
];

/// The tile one enemy body shows. Stat modifiers have no pose and stay on the ready tile.
pub(super) fn status_tile(alive: bool, statuses: &[ActiveStatus]) -> u32 {
    if !alive {
        return KNOCKED_OUT_TILE;
    }
    PRECEDENCE
        .iter()
        .find(|(effect, _)| statuses.iter().any(|status| status.effect == *effect))
        .map_or(READY_TILE, |(_, tile)| *tile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(effect: StatusEffect) -> ActiveStatus {
        ActiveStatus::timed(effect, 2)
    }

    #[test]
    fn each_status_maps_to_its_documented_tile() {
        let table = [
            (StatusEffect::Poison, 1),
            (StatusEffect::Sleep, 2),
            (StatusEffect::Stun, 3),
            (StatusEffect::Silence, 4),
            (StatusEffect::Burn, 5),
            (StatusEffect::Freeze, 6),
            (StatusEffect::Knockback, 7),
            (StatusEffect::Taunt, 8),
            (StatusEffect::DamageReduction, 9),
            (StatusEffect::RedirectDamage, 10),
            (StatusEffect::Haste, 11),
            (StatusEffect::Slow, 12),
            (StatusEffect::Regen, 13),
        ];
        for (effect, tile) in table {
            assert_eq!(status_tile(true, &[with(effect)]), tile, "{effect:?}");
        }
    }

    #[test]
    fn modifiers_and_no_status_stay_ready() {
        assert_eq!(status_tile(true, &[]), READY_TILE);
        for effect in [
            StatusEffect::AttackModifier,
            StatusEffect::DefenseModifier,
            StatusEffect::MagicResistanceModifier,
            StatusEffect::HitChanceModifier,
        ] {
            assert_eq!(status_tile(true, &[with(effect)]), READY_TILE);
        }
    }

    #[test]
    fn knocked_out_beats_every_status_and_the_first_match_wins() {
        assert_eq!(
            status_tile(false, &[with(StatusEffect::Freeze)]),
            KNOCKED_OUT_TILE
        );
        assert_eq!(
            status_tile(
                true,
                &[with(StatusEffect::Regen), with(StatusEffect::Freeze)]
            ),
            6
        );
        assert_eq!(
            status_tile(true, &[with(StatusEffect::Haste), with(StatusEffect::Slow)]),
            12
        );
    }

    #[test]
    fn attack_kinds_map_to_the_two_attack_tiles() {
        assert_eq!(attack_tile(AttackKind::Thrust), 0);
        assert_eq!(attack_tile(AttackKind::Spellcast), 1);
        assert_eq!(
            attack_sheet_path("orc_raider_base").unwrap().to_string(),
            "media/sprites/enemies/v2/orc_raider_base-attack.tsx"
        );
    }

    #[test]
    fn the_sheet_path_sits_under_v2() {
        assert_eq!(
            status_sheet_path("orc_raider_base").unwrap().to_string(),
            "media/sprites/enemies/v2/orc_raider_base-status.tsx"
        );
    }
}
