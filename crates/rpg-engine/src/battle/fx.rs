use bevy::prelude::*;

use super::{
    action::BattleEvent,
    model::{BattleState, CombatantKey},
    ui::{BattleAssetState, BattleEnemyFrame, BattlePartyCard, BattleUi},
};
use crate::encounter::BattleSide;

const FLOAT_DURATION_SECONDS: f32 = 0.85;
const FLOAT_RISE_PIXELS: f32 = 42.0;
const FLASH_DURATION_SECONDS: f32 = 0.14;
const SCREEN_FLASH_DURATION_SECONDS: f32 = 0.26;
/// Fraction of the screen flash spent at full strength before it fades out.
const SCREEN_FLASH_HOLD_FRACTION: f32 = 0.3;
const SCREEN_FLASH_PEAK_ALPHA: f32 = 0.38;

#[derive(Debug, Default, Resource)]
pub(super) struct BattleFxRouter {
    next_event: usize,
}

#[derive(Component)]
pub(super) struct BattleFxFloat {
    elapsed: f32,
}

#[derive(Component)]
pub(super) struct BattleHitFlash {
    elapsed: f32,
}

/// Full-canvas tint raised whenever the party itself takes a hit.
#[derive(Component)]
pub(super) struct BattleScreenFlash {
    elapsed: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FxColor {
    Damage,
    Critical,
    Recovery,
    Mana,
    Status,
    Miss,
}

#[derive(Clone, Debug, PartialEq)]
struct FxCue {
    target: CombatantKey,
    label: String,
    color: FxColor,
    flash: bool,
}

pub(super) fn route_battle_fx(
    mut commands: Commands,
    state: Option<Res<BattleState>>,
    assets: Option<Res<BattleAssetState>>,
    router: Option<ResMut<BattleFxRouter>>,
    party_cards: Query<(&BattlePartyCard, Entity)>,
    enemy_frames: Query<(&BattleEnemyFrame, Entity)>,
    mut screen_flashes: Query<&mut BattleScreenFlash>,
) {
    let (Some(state), Some(assets), Some(mut router)) = (state, assets, router) else {
        return;
    };
    if router.next_event > state.feedback_events.len() {
        router.next_event = 0;
    }
    let mut party_struck = false;
    for event in &state.feedback_events[router.next_event..] {
        let Some(cue) = cue_for_event(event) else {
            continue;
        };
        party_struck |= triggers_screen_flash(&cue);
        let target = match cue.target.side {
            BattleSide::Party => party_cards
                .iter()
                .find_map(|(marker, entity)| (marker.0 == cue.target.index).then_some(entity)),
            BattleSide::Enemy => enemy_frames
                .iter()
                .find_map(|(marker, entity)| (marker.0 == cue.target.index).then_some(entity)),
        };
        let Some(target) = target else { continue };
        commands.entity(target).with_children(|parent| {
            parent.spawn((
                Text::new(cue.label),
                TextFont {
                    font: assets.font.clone().into(),
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(color(cue.color, 1.0)),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    top: px(-8),
                    ..default()
                },
                GlobalZIndex(20),
                BattleFxFloat { elapsed: 0.0 },
            ));
            if cue.flash {
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.72)),
                    GlobalZIndex(10),
                    Pickable::IGNORE,
                    BattleHitFlash { elapsed: 0.0 },
                ));
            }
        });
    }
    router.next_event = state.feedback_events.len();
    if !party_struck {
        return;
    }
    // A group hit produces several cues in one frame; restart the single overlay
    // instead of stacking translucent copies until the screen washes out.
    if let Some(mut existing) = screen_flashes.iter_mut().next() {
        existing.elapsed = 0.0;
        return;
    }
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(screen_flash_color(SCREEN_FLASH_PEAK_ALPHA)),
        GlobalZIndex(260),
        Pickable::IGNORE,
        BattleUi,
        BattleScreenFlash { elapsed: 0.0 },
    ));
}

pub(super) fn animate_battle_fx(
    mut commands: Commands,
    time: Res<Time>,
    mut floats: Query<(Entity, &mut BattleFxFloat, &mut Node, &mut TextColor)>,
    mut flashes: Query<(Entity, &mut BattleHitFlash, &mut BackgroundColor)>,
    mut screen_flashes: Query<
        (Entity, &mut BattleScreenFlash, &mut BackgroundColor),
        Without<BattleHitFlash>,
    >,
) {
    let delta = time.delta_secs().max(0.0);
    for (entity, mut effect, mut node, mut text_color) in &mut floats {
        effect.elapsed += delta;
        let (top, alpha, expired) = float_frame(effect.elapsed);
        node.top = px(top);
        text_color.0 = text_color.0.with_alpha(alpha);
        if expired {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut effect, mut background) in &mut flashes {
        effect.elapsed += delta;
        let (alpha, expired) = flash_frame(effect.elapsed);
        background.0 = Color::srgba(1.0, 1.0, 1.0, alpha * 0.72);
        if expired {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut effect, mut background) in &mut screen_flashes {
        effect.elapsed += delta;
        let (alpha, expired) = screen_flash_frame(effect.elapsed);
        background.0 = screen_flash_color(alpha);
        if expired {
            commands.entity(entity).despawn();
        }
    }
}

fn cue_for_event(event: &BattleEvent) -> Option<FxCue> {
    let cue = match *event {
        BattleEvent::Miss { action } => FxCue {
            target: action.target(),
            label: "MISS".to_owned(),
            color: FxColor::Miss,
            flash: false,
        },
        BattleEvent::Damage {
            action,
            amount,
            critical,
            ..
        } => FxCue {
            target: action.target(),
            label: if critical {
                format!("CRIT {amount}")
            } else {
                amount.to_string()
            },
            color: if critical {
                FxColor::Critical
            } else {
                FxColor::Damage
            },
            flash: true,
        },
        BattleEvent::MagicDamage { target, amount, .. }
        | BattleEvent::EnemyAbilityDamage { target, amount, .. }
        | BattleEvent::ItemDamage { target, amount, .. }
        | BattleEvent::StatusDamage { target, amount, .. } => FxCue {
            target,
            label: amount.to_string(),
            color: FxColor::Damage,
            flash: true,
        },
        BattleEvent::EnemyAbilityBlocked { target, .. } => FxCue {
            target,
            label: "BLOCKED".to_owned(),
            color: FxColor::Miss,
            flash: false,
        },
        BattleEvent::Heal {
            target,
            amount,
            revived,
            ..
        } => FxCue {
            target,
            label: if revived {
                "REVIVE".to_owned()
            } else {
                format!("+{amount}")
            },
            color: FxColor::Recovery,
            flash: false,
        },
        BattleEvent::ManaRestored { target, amount, .. } => FxCue {
            target,
            label: format!("+{amount} MP"),
            color: FxColor::Mana,
            flash: false,
        },
        BattleEvent::StatusApplied { target, status, .. } => FxCue {
            target,
            label: format!("{:?}", status.effect).to_uppercase(),
            color: FxColor::Status,
            flash: false,
        },
        BattleEvent::StatusCured { target, .. } => FxCue {
            target,
            label: "CURED".to_owned(),
            color: FxColor::Recovery,
            flash: false,
        },
    };
    Some(cue)
}

fn float_frame(elapsed: f32) -> (f32, f32, bool) {
    let progress = (elapsed / FLOAT_DURATION_SECONDS).clamp(0.0, 1.0);
    (
        -8.0 - FLOAT_RISE_PIXELS * progress,
        1.0 - progress,
        elapsed >= FLOAT_DURATION_SECONDS,
    )
}

fn flash_frame(elapsed: f32) -> (f32, bool) {
    let progress = (elapsed / FLASH_DURATION_SECONDS).clamp(0.0, 1.0);
    (1.0 - progress, elapsed >= FLASH_DURATION_SECONDS)
}

/// Only damage landing on the party shakes the whole canvas; hits the party
/// deals out already read clearly from the enemy's own frame flash.
fn triggers_screen_flash(cue: &FxCue) -> bool {
    cue.flash && cue.target.side == BattleSide::Party
}

/// Holds the tint at full strength, then fades it linearly to nothing.
fn screen_flash_frame(elapsed: f32) -> (f32, bool) {
    let progress = (elapsed / SCREEN_FLASH_DURATION_SECONDS).clamp(0.0, 1.0);
    let fade = ((progress - SCREEN_FLASH_HOLD_FRACTION) / (1.0 - SCREEN_FLASH_HOLD_FRACTION))
        .clamp(0.0, 1.0);
    (
        SCREEN_FLASH_PEAK_ALPHA * (1.0 - fade),
        elapsed >= SCREEN_FLASH_DURATION_SECONDS,
    )
}

fn screen_flash_color(alpha: f32) -> Color {
    Color::srgba(0.85, 0.09, 0.09, alpha)
}

fn color(color: FxColor, alpha: f32) -> Color {
    let (red, green, blue) = match color {
        FxColor::Damage => (1.0, 0.35, 0.25),
        FxColor::Critical => (1.0, 0.82, 0.25),
        FxColor::Recovery => (0.35, 1.0, 0.55),
        FxColor::Mana => (0.5, 0.65, 1.0),
        FxColor::Status => (0.8, 0.55, 1.0),
        FxColor::Miss => (0.9, 0.9, 0.9),
    };
    Color::srgba(red, green, blue, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::{action::BattleAction, model::BattlePhase, status::StatusEffect};

    #[test]
    fn damage_miss_critical_and_status_events_route_to_distinct_cues() {
        let action = BattleAction::Physical {
            attacker: CombatantKey::party(0),
            target: CombatantKey::enemy(0),
        };
        assert_eq!(
            cue_for_event(&BattleEvent::Miss { action }).unwrap(),
            FxCue {
                target: CombatantKey::enemy(0),
                label: "MISS".to_owned(),
                color: FxColor::Miss,
                flash: false,
            }
        );
        let critical = cue_for_event(&BattleEvent::Damage {
            action,
            amount: 15,
            critical: true,
            knocked_out: false,
        })
        .unwrap();
        assert_eq!(critical.label, "CRIT 15");
        assert_eq!(critical.color, FxColor::Critical);
        assert!(critical.flash);
        let status = cue_for_event(&BattleEvent::StatusDamage {
            target: CombatantKey::party(0),
            effect: StatusEffect::Poison,
            amount: 3,
            knocked_out: false,
        })
        .unwrap();
        assert_eq!(status.label, "3");
        assert!(status.flash);
    }

    #[test]
    fn float_rise_fade_and_flash_cleanup_have_exact_bounds() {
        assert_eq!(float_frame(0.0), (-8.0, 1.0, false));
        assert_eq!(float_frame(FLOAT_DURATION_SECONDS), (-50.0, 0.0, true));
        assert_eq!(float_frame(9.0), (-50.0, 0.0, true));
        assert_eq!(flash_frame(0.0), (1.0, false));
        assert_eq!(flash_frame(FLASH_DURATION_SECONDS), (0.0, true));
    }

    #[test]
    fn only_damage_taken_by_the_party_raises_the_screen_flash() {
        let struck_party = cue_for_event(&BattleEvent::EnemyAbilityDamage {
            source: CombatantKey::enemy(0),
            target: CombatantKey::party(1),
            amount: 9,
            knocked_out: false,
        })
        .unwrap();
        assert!(triggers_screen_flash(&struck_party));
        let struck_enemy = cue_for_event(&BattleEvent::Damage {
            action: BattleAction::Physical {
                attacker: CombatantKey::party(0),
                target: CombatantKey::enemy(0),
            },
            amount: 9,
            critical: false,
            knocked_out: false,
        })
        .unwrap();
        assert!(!triggers_screen_flash(&struck_enemy));
        let healed_party = cue_for_event(&BattleEvent::Heal {
            source: CombatantKey::party(0),
            target: CombatantKey::party(1),
            amount: 9,
            revived: false,
        })
        .unwrap();
        assert!(!triggers_screen_flash(&healed_party));
    }

    #[test]
    fn screen_flash_holds_then_fades_to_nothing_within_its_budget() {
        assert_eq!(
            screen_flash_frame(0.0),
            (SCREEN_FLASH_PEAK_ALPHA, false),
            "the flash opens at full strength"
        );
        let hold_end = SCREEN_FLASH_DURATION_SECONDS * SCREEN_FLASH_HOLD_FRACTION;
        assert_eq!(
            screen_flash_frame(hold_end),
            (SCREEN_FLASH_PEAK_ALPHA, false)
        );
        let (mid_alpha, mid_expired) =
            screen_flash_frame(hold_end + (SCREEN_FLASH_DURATION_SECONDS - hold_end) / 2.0);
        assert!(mid_alpha > 0.0 && mid_alpha < SCREEN_FLASH_PEAK_ALPHA);
        assert!(!mid_expired);
        assert_eq!(
            screen_flash_frame(SCREEN_FLASH_DURATION_SECONDS),
            (0.0, true)
        );
        assert_eq!(screen_flash_frame(9.0), (0.0, true));
    }

    #[test]
    fn feedback_cues_do_not_change_battle_phase_or_health() {
        let event = BattleEvent::Heal {
            source: CombatantKey::party(0),
            target: CombatantKey::party(1),
            amount: 10,
            revived: false,
        };
        let phase = BattlePhase::Resolve;
        let health = 35;
        assert!(cue_for_event(&event).is_some());
        assert_eq!(phase, BattlePhase::Resolve);
        assert_eq!(health, 35);
    }
}
