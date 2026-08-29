//! Persistent status-effect badges for the battle frames.
//!
//! The model has tracked fourteen effects since the resolver landed, but the only thing that ever
//! drew one was the one-shot float raised when it was applied — so a player who looked away for a
//! second could not tell who was still poisoned or silenced. This is the port of the pinned
//! engine's badge (`battle_party_panel_renderer.py:136-151`) and its palette
//! (`battle_renderer_constants.py:32-42`).
//!
//! Two deliberate widenings of the source:
//!
//! - The source palette covers nine effects and silently draws nothing for the rest. Every effect
//!   the Rust model can carry gets a badge here, because an unrendered status is exactly the bug
//!   this closes.
//! - The source's modifier entry is `DEF_UP` only. The Rust model's modifier effects are
//!   direction-agnostic — one variant covers both a rally and a weakening — so the potency picks
//!   the sign, the same way [`super::fx::status_cue`] picks the buff or debuff sound.

use bevy::prelude::Color;

use super::status::{ActiveStatus, StatusEffect, StatusPotency};

/// Badges drawn per combatant before the rest are dropped.
///
/// The source draws exactly one and hides the rest. Three is the deliberate widening: knowing a
/// member is *both* poisoned and silenced is the whole point of a persistent badge, and one slot
/// cannot say that. The cap exists because the narrowest frame a stack sits on is a 52px small
/// enemy, where a fourth pill would bury the sprite it is annotating.
pub(super) const MAX_BADGES: usize = 3;

/// One rendered pill: what it says and the two colors it says it in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct StatusBadge {
    pub(super) label: &'static str,
    pub(super) fill: Color,
    pub(super) ink: Color,
}

const fn badge(label: &'static str, fill: (u8, u8, u8), ink: (u8, u8, u8)) -> StatusBadge {
    StatusBadge {
        label,
        fill: Color::srgb_u8(fill.0, fill.1, fill.2),
        ink: Color::srgb_u8(ink.0, ink.1, ink.2),
    }
}

/// The source's `DEF_UP` teal, reused for every modifier reading in the wearer's favour.
const BOON: ((u8, u8, u8), (u8, u8, u8)) = ((40, 110, 110), (170, 230, 230));
/// The mirror of [`BOON`] for a modifier working against its wearer.
const BANE: ((u8, u8, u8), (u8, u8, u8)) = ((110, 46, 46), (240, 180, 180));

/// Whether a modifier status helps the combatant wearing it.
///
/// Mirrors the same judgement [`super::fx::status_cue`] makes when choosing between the buff and
/// debuff cues: a multiplier at or above 1.0 raises the stat, and a damage reduction is always in
/// the wearer's favour.
fn beneficial(potency: StatusPotency) -> bool {
    match potency {
        StatusPotency::Multiplier(factor) => factor >= 1.0,
        StatusPotency::Reduction(_) => true,
        StatusPotency::None | StatusPotency::DamagePerTurn(_) | StatusPotency::Redirect(_) => false,
    }
}

/// The pill one active status wears, or `None` when it is not worth a badge.
pub(super) fn badge_for(status: ActiveStatus) -> Option<StatusBadge> {
    let signed = |up: &'static str, down: &'static str| {
        if beneficial(status.potency) {
            badge(up, BOON.0, BOON.1)
        } else {
            badge(down, BANE.0, BANE.1)
        }
    };
    Some(match status.effect {
        StatusEffect::Poison => badge("PSN", (51, 102, 51), (170, 255, 170)),
        StatusEffect::Sleep => badge("zzz", (68, 68, 170), (204, 204, 255)),
        StatusEffect::Stun => badge("STN", (120, 90, 20), (255, 220, 100)),
        StatusEffect::Silence => badge("SIL", (100, 60, 100), (220, 180, 220)),
        StatusEffect::Burn => badge("BRN", (140, 40, 40), (255, 170, 120)),
        StatusEffect::Freeze => badge("FRZ", (40, 80, 140), (180, 220, 255)),
        StatusEffect::Knockback => badge("KBK", (90, 90, 120), (210, 210, 230)),
        StatusEffect::Taunt => badge("TNT", (150, 60, 30), (255, 180, 100)),
        StatusEffect::AttackModifier => signed("ATK+", "ATK-"),
        StatusEffect::DefenseModifier => signed("DEF+", "DEF-"),
        StatusEffect::MagicResistanceModifier => signed("RES+", "RES-"),
        StatusEffect::HitChanceModifier => signed("HIT+", "HIT-"),
        StatusEffect::DamageReduction => badge("GRD", BOON.0, BOON.1),
        StatusEffect::RedirectDamage => badge("CVR", (90, 70, 130), (215, 195, 245)),
    })
}

/// Every badge a combatant should wear, in application order, capped at [`MAX_BADGES`].
///
/// Application order is what the source shows — it draws `status_effects[0]` — so the oldest
/// affliction stays put instead of the row reshuffling every time something else lands.
pub(super) fn badges(statuses: &[ActiveStatus]) -> Vec<StatusBadge> {
    statuses
        .iter()
        .filter_map(|status| badge_for(*status))
        .take(MAX_BADGES)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tracked_effect_earns_a_badge() {
        // The gap this closes is an effect the model tracks and the screen never shows, so the
        // mapping has to be total.
        for effect in [
            StatusEffect::Poison,
            StatusEffect::Sleep,
            StatusEffect::Stun,
            StatusEffect::Silence,
            StatusEffect::Burn,
            StatusEffect::Freeze,
            StatusEffect::Knockback,
            StatusEffect::Taunt,
            StatusEffect::AttackModifier,
            StatusEffect::DefenseModifier,
            StatusEffect::MagicResistanceModifier,
            StatusEffect::HitChanceModifier,
            StatusEffect::DamageReduction,
            StatusEffect::RedirectDamage,
        ] {
            assert!(
                badge_for(ActiveStatus::persistent(effect)).is_some(),
                "{effect:?} has no badge"
            );
        }
    }

    #[test]
    fn the_source_palette_is_reproduced_exactly() {
        // `battle_renderer_constants.py:32-42`, verbatim.
        for (effect, label, fill, ink) in [
            (StatusEffect::Poison, "PSN", (51, 102, 51), (170, 255, 170)),
            (StatusEffect::Sleep, "zzz", (68, 68, 170), (204, 204, 255)),
            (StatusEffect::Stun, "STN", (120, 90, 20), (255, 220, 100)),
            (
                StatusEffect::Silence,
                "SIL",
                (100, 60, 100),
                (220, 180, 220),
            ),
            (StatusEffect::Burn, "BRN", (140, 40, 40), (255, 170, 120)),
            (StatusEffect::Freeze, "FRZ", (40, 80, 140), (180, 220, 255)),
            (
                StatusEffect::Knockback,
                "KBK",
                (90, 90, 120),
                (210, 210, 230),
            ),
            (StatusEffect::Taunt, "TNT", (150, 60, 30), (255, 180, 100)),
        ] {
            assert_eq!(
                badge_for(ActiveStatus::persistent(effect)),
                Some(badge(label, fill, ink)),
                "{effect:?}"
            );
        }
        assert_eq!(
            badge_for(ActiveStatus::modifier(
                StatusEffect::DefenseModifier,
                3,
                1.25
            )),
            Some(badge("DEF+", BOON.0, BOON.1)),
            "the source's lone modifier entry is DEF_UP, in this teal"
        );
    }

    #[test]
    fn a_modifiers_potency_decides_which_way_its_badge_reads() {
        let raised = badge_for(ActiveStatus::modifier(StatusEffect::AttackModifier, 3, 1.5))
            .expect("attack modifier badges");
        let lowered = badge_for(ActiveStatus::modifier(StatusEffect::AttackModifier, 3, 0.5))
            .expect("attack modifier badges");
        assert_eq!(raised.label, "ATK+");
        assert_eq!(lowered.label, "ATK-");
        assert_ne!(raised.fill, lowered.fill);
        // A flat damage reduction always favours its wearer, whatever the number.
        assert_eq!(
            badge_for(ActiveStatus::reduction(2, 0.3)),
            Some(badge("GRD", BOON.0, BOON.1))
        );
    }

    #[test]
    fn badges_keep_application_order_and_stop_at_the_cap() {
        let statuses = [
            ActiveStatus::persistent(StatusEffect::Poison),
            ActiveStatus::persistent(StatusEffect::Silence),
            ActiveStatus::persistent(StatusEffect::Burn),
            ActiveStatus::persistent(StatusEffect::Freeze),
        ];
        let drawn = badges(&statuses);
        assert_eq!(drawn.len(), MAX_BADGES);
        assert_eq!(
            drawn.iter().map(|badge| badge.label).collect::<Vec<_>>(),
            vec!["PSN", "SIL", "BRN"],
            "the oldest affliction stays in place; the newest is the one dropped"
        );
        assert!(badges(&[]).is_empty());
    }
}
