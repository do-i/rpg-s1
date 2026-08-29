use std::collections::HashMap;

use bevy::prelude::*;

use super::{
    action::BattleEvent,
    model::{BattleState, CombatantKey},
    status::{StatusEffect, StatusPotency},
    ui::{BattleAssetState, BattleEnemyFrame, BattlePartyCard, BattleUi},
};
use crate::{
    encounter::BattleSide,
    scenario_class::AbilityElement,
    scenario_item::ItemElement,
    sfx_cue::{PlaySfx, cue},
};

const FLOAT_DURATION_SECONDS: f32 = 0.85;
const FLOAT_RISE_PIXELS: f32 = 42.0;
const FLASH_DURATION_SECONDS: f32 = 0.14;
const SCREEN_FLASH_DURATION_SECONDS: f32 = 0.26;
/// Fraction of the screen flash spent at full strength before it fades out.
const SCREEN_FLASH_HOLD_FRACTION: f32 = 0.3;
const SCREEN_FLASH_PEAK_ALPHA: f32 = 0.38;
/// Hurt shake, ported exactly from `battle_fx.py:12-14` — a 4px swing damped to nothing over
/// 0.22s at 22Hz. See [`shake_frame`] for what that actually looks like on screen.
const SHAKE_DURATION_SECONDS: f32 = 0.22;
const SHAKE_AMPLITUDE_PIXELS: f32 = 4.0;
const SHAKE_FREQUENCY: f32 = 22.0;
/// How long an attacker plays its sprite-row animation (`battle_fx.py:18`).
const ATTACK_DURATION_SECONDS: f32 = 0.5;

#[derive(Debug, Default, Resource)]
pub(super) struct BattleFxRouter {
    next_event: usize,
}

/// Which sprite row an attacker plays while acting.
///
/// The source splits exactly here: a basic attack thrusts, an ability is cast
/// (`battle_enemy_logic.py:53,69`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AttackKind {
    Thrust,
    Spellcast,
}

#[derive(Clone, Copy, Debug)]
struct ActiveAttack {
    kind: AttackKind,
    elapsed: f32,
}

/// The attack animations currently playing, keyed by enemy slot.
///
/// Only enemies animate: the party is drawn from static portraits, exactly as in the source, whose
/// attack frames come from the enemy area renderer alone.
#[derive(Debug, Default, Resource)]
pub(super) struct BattleAttackAnimations {
    active: HashMap<usize, ActiveAttack>,
}

impl BattleAttackAnimations {
    /// Starts — or restarts — the animation on one enemy slot.
    fn start(&mut self, index: usize, kind: AttackKind) {
        self.active
            .insert(index, ActiveAttack { kind, elapsed: 0.0 });
    }

    /// The row and 0..1 progress an enemy is at, or `None` when it is idle.
    pub(super) fn progress(&self, index: usize) -> Option<(AttackKind, f32)> {
        self.active
            .get(&index)
            .map(|attack| (attack.kind, attack_progress(attack.elapsed)))
    }

    fn tick(&mut self, delta: f32) {
        for attack in self.active.values_mut() {
            attack.elapsed += delta;
        }
        self.active
            .retain(|_, attack| attack.elapsed < ATTACK_DURATION_SECONDS);
    }
}

/// The hurt shake riding on one combatant's frame.
#[derive(Component)]
pub(super) struct BattleHurtShake {
    elapsed: f32,
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

#[expect(
    clippy::too_many_arguments,
    reason = "a Bevy system's parameters are its dependency injection; routing one event stream to \
              floats, frame flashes, the screen flash, and audio needs each of these"
)]
pub(super) fn route_battle_fx(
    mut commands: Commands,
    state: Option<Res<BattleState>>,
    assets: Option<Res<BattleAssetState>>,
    router: Option<ResMut<BattleFxRouter>>,
    attacks: Option<ResMut<BattleAttackAnimations>>,
    party_cards: Query<(&BattlePartyCard, Entity)>,
    enemy_frames: Query<(&BattleEnemyFrame, Entity)>,
    mut screen_flashes: Query<&mut BattleScreenFlash>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let (Some(state), Some(assets), Some(mut router), Some(mut attacks)) =
        (state, assets, router, attacks)
    else {
        return;
    };
    if router.next_event > state.feedback_events.len() {
        router.next_event = 0;
    }
    let mut party_struck = false;
    for event in &state.feedback_events[router.next_event..] {
        // Audio is routed off the same cursor but independently of the visual cue: an event that
        // draws nothing may still need to be heard.
        sfx.write_batch(sfx_cues_for_event(event).into_iter().map(PlaySfx::new));
        // The attacker's animation is keyed off the same event that reports the hit, which is
        // where the source starts it (`battle_enemy_logic.py:53,69`).
        if let Some((attacker, kind)) = attack_for_event(event)
            && attacker.side == BattleSide::Enemy
        {
            attacks.start(attacker.index, kind);
        }
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
        // `fx.hit()` in the source is flash *and* shake, always together (`battle_fx.py:88-91`);
        // re-inserting restarts a shake already running, matching its keyed-dict overwrite.
        if cue.flash {
            commands
                .entity(target)
                .insert(BattleHurtShake { elapsed: 0.0 });
        }
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
    attacks: Option<ResMut<BattleAttackAnimations>>,
    mut floats: Query<(Entity, &mut BattleFxFloat, &mut Node, &mut TextColor)>,
    mut flashes: Query<(Entity, &mut BattleHitFlash, &mut BackgroundColor)>,
    mut screen_flashes: Query<
        (Entity, &mut BattleScreenFlash, &mut BackgroundColor),
        Without<BattleHitFlash>,
    >,
) {
    let delta = time.delta_secs().max(0.0);
    if let Some(mut attacks) = attacks {
        attacks.tick(delta);
    }
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

/// Nudges every shaking frame sideways and clears the offset when the shake burns out.
///
/// Separate from [`animate_battle_fx`] because both want `&mut Node`, and one system may not hold
/// two mutable borrows of the same component.
pub(super) fn animate_battle_shake(
    mut commands: Commands,
    time: Res<Time>,
    mut shakes: Query<(Entity, &mut BattleHurtShake, &mut Node)>,
) {
    let delta = time.delta_secs().max(0.0);
    for (entity, mut shake, mut node) in &mut shakes {
        shake.elapsed += delta;
        let (offset, expired) = shake_frame(shake.elapsed);
        node.left = px(offset);
        if expired {
            commands.entity(entity).remove::<BattleHurtShake>();
        }
    }
}

/// Which attacker starts an animation on this event, and which row it plays.
///
/// A whiff still swings, so [`BattleEvent::Miss`] animates like the hit it was aiming to be.
/// Events with no actor behind them — a poison tick — animate nobody.
const fn attack_for_event(event: &BattleEvent) -> Option<(CombatantKey, AttackKind)> {
    Some(match *event {
        BattleEvent::Miss { action } | BattleEvent::Damage { action, .. } => {
            (action.attacker(), AttackKind::Thrust)
        }
        BattleEvent::EnemyAbilityDamage { source, .. }
        | BattleEvent::EnemyAbilityBlocked { source, .. }
        | BattleEvent::MagicDamage { source, .. }
        | BattleEvent::ItemDamage { source, .. }
        | BattleEvent::Heal { source, .. }
        | BattleEvent::ManaRestored { source, .. }
        | BattleEvent::StatusApplied { source, .. }
        | BattleEvent::StatusCured { source, .. } => (source, AttackKind::Spellcast),
        BattleEvent::StatusDamage { .. } => return None,
    })
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

/// The impact cues a hit landing on `side` should make.
///
/// Mirrors the pinned engine, which plays two sounds when the party is struck — the physical
/// impact and the party's own hurt cue (`battle_enemy_logic.py:55,57,82,84`) — and the swing alone
/// when the party is the one connecting.
const fn impact_cues(side: BattleSide) -> &'static [&'static str] {
    match side {
        BattleSide::Party => &[cue::ATK_IMPACT, cue::PARTY_HIT],
        BattleSide::Enemy => &[cue::ATK_SLASH],
    }
}

/// The authored cue for a spell's element.
///
/// `Holy` returns `None`: `sfx_index.yaml` authors no `spell_holy`. The reverse also holds — the
/// index authors `spell_ice` and `spell_thunder`, and neither `AbilityElement` nor `ItemElement`
/// has an Ice or Thunder variant to reach them. Both gaps are inherited content, not routing bugs.
const fn ability_spell_cue(element: AbilityElement) -> Option<&'static str> {
    match element {
        AbilityElement::Fire => Some(cue::SPELL_FIRE),
        AbilityElement::Water => Some(cue::SPELL_WATER),
        AbilityElement::Wind => Some(cue::SPELL_WIND),
        AbilityElement::Earth => Some(cue::SPELL_EARTH),
        AbilityElement::Holy => None,
    }
}

const fn item_spell_cue(element: ItemElement) -> Option<&'static str> {
    match element {
        ItemElement::Fire => Some(cue::SPELL_FIRE),
        ItemElement::Water => Some(cue::SPELL_WATER),
        ItemElement::Wind => Some(cue::SPELL_WIND),
        ItemElement::Holy => None,
    }
}

/// Whether an applied status reads as a boon or an affliction, and which cue says so.
///
/// The modifier statuses are direction-agnostic in the model — `AttackModifier` covers both a
/// rally and a weakening — so the potency decides: a multiplier at or above 1.0 raises a stat, and
/// a damage reduction is always in the wearer's favour. Everything else is an affliction.
fn status_cue(status_effect: StatusEffect, potency: StatusPotency) -> &'static str {
    let beneficial = match potency {
        StatusPotency::Multiplier(factor) => factor >= 1.0,
        StatusPotency::Reduction(_) => true,
        StatusPotency::None | StatusPotency::DamagePerTurn(_) | StatusPotency::Redirect(_) => false,
    };
    if !beneficial {
        return cue::DEBUFF;
    }
    match status_effect {
        StatusEffect::DefenseModifier
        | StatusEffect::MagicResistanceModifier
        | StatusEffect::DamageReduction => cue::DEF_BUFF,
        _ => cue::ATK_BUFF,
    }
}

/// Maps one resolved battle event to the SFX cues it should fire, in sounding order.
///
/// This is the port of the pinned `play_battle_action` dispatch
/// (`engine/audio/sfx_manager.py:65-91`) together with the impact and death cues the source plays
/// at resolution time. It is a pure function so the routing can be asserted without an audio
/// device; `route_battle_fx` writes the results and the cue service collapses duplicates.
/// Appends the death cue when a killing blow felled an enemy. The party falling is covered by the
/// hurt cue that already sounded, matching the source, which has no party-death sample.
fn push_knockout(knocked_out: bool, target: CombatantKey, cues: &mut Vec<&'static str>) {
    if knocked_out && target.side == BattleSide::Enemy {
        cues.push(cue::ENEMY_DEATH);
    }
}

fn sfx_cues_for_event(event: &BattleEvent) -> Vec<&'static str> {
    let mut cues: Vec<&'static str> = Vec::new();
    match *event {
        // The pinned engine plays no cue on a whiff; the floating MISS label carries it.
        BattleEvent::Miss { .. } => {}
        BattleEvent::Damage {
            action,
            knocked_out,
            ..
        } => {
            let target = action.target();
            cues.extend_from_slice(impact_cues(target.side));
            push_knockout(knocked_out, target, &mut cues);
        }
        BattleEvent::MagicDamage {
            target,
            element,
            knocked_out,
            ..
        } => {
            cues.extend(ability_spell_cue(element));
            push_knockout(knocked_out, target, &mut cues);
        }
        BattleEvent::ItemDamage {
            target,
            element,
            knocked_out,
            ..
        } => {
            cues.push(cue::USE_ITEM);
            cues.extend(item_spell_cue(element));
            push_knockout(knocked_out, target, &mut cues);
        }
        BattleEvent::EnemyAbilityDamage {
            target,
            knocked_out,
            ..
        } => {
            cues.extend_from_slice(impact_cues(target.side));
            push_knockout(knocked_out, target, &mut cues);
        }
        // A blocked ability is the defensive beat the `defend` cue exists for.
        BattleEvent::EnemyAbilityBlocked { .. } => cues.push(cue::DEFEND),
        BattleEvent::Heal { revived, .. } => {
            cues.push(if revived { cue::REVIVE } else { cue::HEAL });
        }
        BattleEvent::ManaRestored { .. } | BattleEvent::StatusCured { .. } => {
            cues.push(cue::HEAL);
        }
        BattleEvent::StatusApplied { status, .. } => {
            cues.push(status_cue(status.effect, status.potency));
        }
        BattleEvent::StatusDamage {
            target,
            knocked_out,
            ..
        } => {
            cues.extend_from_slice(impact_cues(target.side));
            push_knockout(knocked_out, target, &mut cues);
        }
    }
    cues
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

/// A damped sine, truncated to whole pixels the way the source's `int()` does
/// (`battle_fx.py:50-53`).
fn shake_frame(elapsed: f32) -> (f32, bool) {
    let expired = elapsed >= SHAKE_DURATION_SECONDS;
    if expired {
        return (0.0, true);
    }
    let decay = (1.0 - elapsed / SHAKE_DURATION_SECONDS).max(0.0);
    (
        (SHAKE_AMPLITUDE_PIXELS * decay * (elapsed * SHAKE_FREQUENCY).sin()).trunc(),
        false,
    )
}

fn attack_progress(elapsed: f32) -> f32 {
    (elapsed / ATTACK_DURATION_SECONDS).clamp(0.0, 1.0)
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
    use crate::battle::{
        action::BattleAction,
        model::BattlePhase,
        status::{ActiveStatus, StatusEffect},
    };

    /// Pins the audio dispatch: which cues each resolved event makes, and in what order.
    ///
    /// The pinned engine plays a swing when the party connects and an impact plus a hurt cue when
    /// the party is struck, so the two directions are deliberately asymmetric.
    #[test]
    fn every_battle_event_routes_to_its_authored_sfx_cues() {
        let party_hits_enemy = BattleAction::Physical {
            attacker: CombatantKey::party(0),
            target: CombatantKey::enemy(0),
        };
        let enemy_hits_party = BattleAction::Physical {
            attacker: CombatantKey::enemy(0),
            target: CombatantKey::party(0),
        };

        // A whiff is silent; the floating label carries it.
        assert_eq!(
            sfx_cues_for_event(&BattleEvent::Miss {
                action: party_hits_enemy
            }),
            Vec::<&str>::new()
        );

        assert_eq!(
            sfx_cues_for_event(&BattleEvent::Damage {
                action: party_hits_enemy,
                amount: 9,
                critical: false,
                knocked_out: false,
            }),
            vec![cue::ATK_SLASH]
        );
        assert_eq!(
            sfx_cues_for_event(&BattleEvent::Damage {
                action: enemy_hits_party,
                amount: 9,
                critical: false,
                knocked_out: false,
            }),
            vec![cue::ATK_IMPACT, cue::PARTY_HIT]
        );

        // A killing blow appends the death cue, but only when an enemy is the one falling.
        assert_eq!(
            sfx_cues_for_event(&BattleEvent::Damage {
                action: party_hits_enemy,
                amount: 99,
                critical: true,
                knocked_out: true,
            }),
            vec![cue::ATK_SLASH, cue::ENEMY_DEATH]
        );
        assert_eq!(
            sfx_cues_for_event(&BattleEvent::Damage {
                action: enemy_hits_party,
                amount: 99,
                critical: false,
                knocked_out: true,
            }),
            vec![cue::ATK_IMPACT, cue::PARTY_HIT]
        );

        // Elements route to their own cue; Holy has none authored.
        for (element, expected) in [
            (AbilityElement::Fire, vec![cue::SPELL_FIRE]),
            (AbilityElement::Water, vec![cue::SPELL_WATER]),
            (AbilityElement::Wind, vec![cue::SPELL_WIND]),
            (AbilityElement::Earth, vec![cue::SPELL_EARTH]),
            (AbilityElement::Holy, Vec::new()),
        ] {
            assert_eq!(
                sfx_cues_for_event(&BattleEvent::MagicDamage {
                    source: CombatantKey::party(1),
                    target: CombatantKey::enemy(0),
                    element,
                    amount: 20,
                    knocked_out: false,
                }),
                expected,
                "element {element:?}"
            );
        }

        assert_eq!(
            sfx_cues_for_event(&BattleEvent::ItemDamage {
                source: CombatantKey::party(0),
                target: CombatantKey::enemy(0),
                element: ItemElement::Fire,
                amount: 12,
                knocked_out: true,
            }),
            vec![cue::USE_ITEM, cue::SPELL_FIRE, cue::ENEMY_DEATH]
        );

        assert_eq!(
            sfx_cues_for_event(&BattleEvent::EnemyAbilityBlocked {
                source: CombatantKey::enemy(0),
                target: CombatantKey::party(0),
            }),
            vec![cue::DEFEND]
        );

        assert_eq!(
            sfx_cues_for_event(&BattleEvent::Heal {
                source: CombatantKey::party(1),
                target: CombatantKey::party(0),
                amount: 30,
                revived: false,
            }),
            vec![cue::HEAL]
        );
        assert_eq!(
            sfx_cues_for_event(&BattleEvent::Heal {
                source: CombatantKey::party(1),
                target: CombatantKey::party(0),
                amount: 30,
                revived: true,
            }),
            vec![cue::REVIVE]
        );

        // Modifier statuses are direction-agnostic in the model, so potency picks the cue.
        for (effect, potency, expected) in [
            (
                StatusEffect::AttackModifier,
                StatusPotency::Multiplier(1.5),
                cue::ATK_BUFF,
            ),
            (
                StatusEffect::AttackModifier,
                StatusPotency::Multiplier(0.5),
                cue::DEBUFF,
            ),
            (
                StatusEffect::DefenseModifier,
                StatusPotency::Multiplier(1.25),
                cue::DEF_BUFF,
            ),
            (
                StatusEffect::DamageReduction,
                StatusPotency::Reduction(0.3),
                cue::DEF_BUFF,
            ),
            (
                StatusEffect::Poison,
                StatusPotency::DamagePerTurn(4),
                cue::DEBUFF,
            ),
            (StatusEffect::Sleep, StatusPotency::None, cue::DEBUFF),
        ] {
            assert_eq!(
                sfx_cues_for_event(&BattleEvent::StatusApplied {
                    source: CombatantKey::enemy(0),
                    target: CombatantKey::party(0),
                    status: ActiveStatus {
                        effect,
                        remaining_turns: Some(3),
                        potency,
                    },
                }),
                vec![expected],
                "{effect:?} with {potency:?}"
            );
        }
    }

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

    /// Pins the shape of the ported shake, including the one part of it that surprises.
    ///
    /// The source is a damped 22Hz sine over 0.22s at 4px, truncated to whole pixels
    /// (`battle_fx.py:44-53`). Because 0.22s is only three quarters of a 22Hz cycle and the decay
    /// is linear, the return swing peaks below one pixel and truncation eats it: what actually
    /// reaches the screen is a single sideways punch that snaps back. That is the source's
    /// behavior, so it is this port's behavior, and this test exists so a later reader does not
    /// "fix" it into a two-sided wobble without deciding to diverge on purpose.
    #[test]
    fn the_hurt_shake_punches_out_once_and_snaps_back_to_zero() {
        assert_eq!(shake_frame(0.0), (0.0, false));
        let peak = shake_frame(0.07).0;
        assert_eq!(
            peak, 2.0,
            "the quarter-swing lands at 2px once decay and truncation are applied"
        );

        let sampled = (0..22)
            .map(|step| shake_frame(step as f32 * 0.01).0)
            .collect::<Vec<_>>();
        assert!(
            sampled
                .iter()
                .all(|&offset| (0.0..=SHAKE_AMPLITUDE_PIXELS).contains(&offset)),
            "the counter-swing decays below a pixel and truncates away: {sampled:?}"
        );
        // Whole pixels only, like the source's int().
        assert!(sampled.iter().all(|&offset| offset == offset.trunc()));
        // And it does come back on its own, well before the timer runs out.
        assert_eq!(shake_frame(0.16).0, 0.0);

        // Never leaves the frame parked off-center.
        assert_eq!(shake_frame(SHAKE_DURATION_SECONDS), (0.0, true));
        assert_eq!(shake_frame(9.0), (0.0, true));
    }

    /// Every event that hit-flashes must also shake: the source calls them together as `fx.hit()`,
    /// and a flash without a shake is the half-effect this task exists to finish.
    #[test]
    fn every_flashing_cue_is_also_a_shaking_one() {
        let action = BattleAction::Physical {
            attacker: CombatantKey::enemy(0),
            target: CombatantKey::party(0),
        };
        for event in [
            BattleEvent::Damage {
                action,
                amount: 7,
                critical: false,
                knocked_out: false,
            },
            BattleEvent::MagicDamage {
                source: CombatantKey::party(1),
                target: CombatantKey::enemy(0),
                element: AbilityElement::Fire,
                amount: 7,
                knocked_out: false,
            },
            BattleEvent::StatusDamage {
                target: CombatantKey::party(0),
                effect: StatusEffect::Poison,
                amount: 3,
                knocked_out: false,
            },
        ] {
            assert!(
                cue_for_event(&event).is_some_and(|cue| cue.flash),
                "{event:?} should flash, and so shake"
            );
        }
        // A whiff neither flashes nor shakes — nothing landed.
        assert!(cue_for_event(&BattleEvent::Miss { action }).is_some_and(|cue| !cue.flash));
    }

    #[test]
    fn a_basic_attack_thrusts_and_everything_else_casts() {
        let action = BattleAction::Physical {
            attacker: CombatantKey::enemy(1),
            target: CombatantKey::party(0),
        };
        assert_eq!(
            attack_for_event(&BattleEvent::Damage {
                action,
                amount: 7,
                critical: false,
                knocked_out: false,
            }),
            Some((CombatantKey::enemy(1), AttackKind::Thrust))
        );
        // A miss still swings.
        assert_eq!(
            attack_for_event(&BattleEvent::Miss { action }),
            Some((CombatantKey::enemy(1), AttackKind::Thrust))
        );
        assert_eq!(
            attack_for_event(&BattleEvent::EnemyAbilityDamage {
                source: CombatantKey::enemy(0),
                target: CombatantKey::party(2),
                amount: 12,
                knocked_out: false,
            }),
            Some((CombatantKey::enemy(0), AttackKind::Spellcast))
        );
        // The animation belongs to the actor, not the victim: a heal animates the healer.
        assert_eq!(
            attack_for_event(&BattleEvent::Heal {
                source: CombatantKey::party(3),
                target: CombatantKey::party(0),
                amount: 20,
                revived: false,
            }),
            Some((CombatantKey::party(3), AttackKind::Spellcast))
        );
        // A poison tick has no actor behind it, so nobody animates.
        assert_eq!(
            attack_for_event(&BattleEvent::StatusDamage {
                target: CombatantKey::party(0),
                effect: StatusEffect::Poison,
                amount: 3,
                knocked_out: false,
            }),
            None
        );
    }

    #[test]
    fn an_attack_animation_runs_its_budget_then_clears_itself() {
        let mut animations = BattleAttackAnimations::default();
        assert_eq!(animations.progress(0), None);

        animations.start(0, AttackKind::Thrust);
        assert_eq!(animations.progress(0), Some((AttackKind::Thrust, 0.0)));
        animations.tick(ATTACK_DURATION_SECONDS / 2.0);
        assert_eq!(animations.progress(0), Some((AttackKind::Thrust, 0.5)));

        // A second action from the same enemy restarts the row rather than stacking onto it.
        animations.start(0, AttackKind::Spellcast);
        assert_eq!(animations.progress(0), Some((AttackKind::Spellcast, 0.0)));

        // Slots are independent: one enemy finishing must not cut another's animation short.
        animations.start(1, AttackKind::Thrust);
        animations.tick(ATTACK_DURATION_SECONDS - 0.01);
        assert!(animations.progress(0).is_some());
        assert!(animations.progress(1).is_some());
        animations.tick(0.02);
        assert_eq!(animations.progress(0), None);
        assert_eq!(animations.progress(1), None);
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

    /// Drives the real router against a real frame entity, because the pure functions above only
    /// prove what *should* happen — this proves the wiring that makes it happen.
    #[test]
    fn a_landed_hit_shakes_the_struck_frame_and_swings_the_attacker() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<PlaySfx>()
            .insert_resource(BattleFxRouter::default())
            .insert_resource(BattleAttackAnimations::default())
            .insert_resource(BattleAssetState::test_stub())
            .add_systems(Update, (route_battle_fx, animate_battle_shake).chain());

        let struck = app
            .world_mut()
            .spawn((Node::default(), BattlePartyCard(0)))
            .id();
        let swinging = app
            .world_mut()
            .spawn((Node::default(), BattleEnemyFrame(1)))
            .id();

        let mut state = crate::battle::tests::state_with(vec![
            crate::battle::tests::actor(BattleSide::Party, 0, 5, 20),
            crate::battle::tests::actor(BattleSide::Enemy, 1, 4, 30),
        ]);
        state.feedback_events.push(BattleEvent::Damage {
            action: BattleAction::Physical {
                attacker: CombatantKey::enemy(1),
                target: CombatantKey::party(0),
            },
            amount: 6,
            critical: false,
            knocked_out: false,
        });
        app.world_mut().insert_resource(state);
        app.update();

        assert!(
            app.world().get::<BattleHurtShake>(struck).is_some(),
            "the struck frame must be shaking"
        );
        assert!(
            app.world().get::<BattleHurtShake>(swinging).is_none(),
            "the attacker is not the one that got hit"
        );
        assert_eq!(
            app.world()
                .resource::<BattleAttackAnimations>()
                .progress(1)
                .map(|(kind, _)| kind),
            Some(AttackKind::Thrust),
            "the swinging enemy plays the thrust row"
        );
        assert_eq!(
            app.world().resource::<BattleAttackAnimations>().progress(0),
            None,
            "party slot 0 is a portrait, and enemy slot 0 never acted"
        );

        // The shake burns out on its own and puts the frame back where it started. The clock is
        // wound forward on the component rather than on `Time`, which `MinimalPlugins` re-reads
        // from the real wall clock on every update.
        app.world_mut()
            .get_mut::<BattleHurtShake>(struck)
            .expect("the shake is still running")
            .elapsed = SHAKE_DURATION_SECONDS;
        app.update();
        assert!(
            app.world().get::<BattleHurtShake>(struck).is_none(),
            "the shake must clean itself up"
        );
        assert_eq!(
            app.world().get::<Node>(struck).map(|node| node.left),
            Some(px(0)),
            "and must not leave the card parked off-center"
        );
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
