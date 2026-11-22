//! The cue that tells the player an unseen field enemy found them.
//!
//! Field enemies are never drawn, so a battle would otherwise start out of nowhere. On contact a
//! "!" pops over the player with a short sting, and the World holds still for a brief beat before
//! the battle flash begins. Variant B adds a softer warning when an unseen enemy starts chasing.
//!
//! Everything tunable for user acceptance lives in [`ContactCueSettings`], read once at startup
//! from `RPG_S1_CUE_VARIANT` and `RPG_S1_CUE_BEAT`, so the switches can later become a real option
//! or be deleted in one place.
//!
//! Nothing here touches `GameplayRng`. The beat freezes the World completely (enemies, NPCs, the
//! player and contact) rather than letting it run longer, so the gameplay stream consumed between
//! contact and battle is the same at every beat length.

use std::ffi::OsString;

use bevy::{asset::AssetServer, prelude::*, sprite::Text2dShadow};

use crate::{
    game_state::GameState,
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    sfx_cue::{PlaySfx, cue},
    world_player::{CHARACTER_SPRITE_SIZE, WorldPlayer, WorldPlayerMotion},
};

use super::{handoff::BattleTransition, simulation::enemy_top_left};

/// The default pause between the contact cue and the battle flash.
pub(crate) const DEFAULT_BEAT_SECONDS: f32 = 0.5;
/// The longest beat the environment switch accepts.
pub(crate) const MAX_BEAT_SECONDS: f32 = 1.5;

/// The contact sting: a short, hard block hit that reads as "!".
pub(super) const CONTACT_STING: &str = cue::DEFEND;
/// The chase warning: a soft whoosh, deliberately quieter than the contact sting.
pub(super) const CHASE_WARNING_SOUND: &str = cue::MISS;
const CHASE_WARNING_GAIN_PERCENT: u16 = 45;
/// Several enemies starting a chase together, or one dancing on the edge of its range, give one
/// warning per this many seconds.
pub(super) const CHASE_WARNING_COOLDOWN_SECONDS: f32 = 2.0;

const CUE_Z: f32 = 500.0;
const CONTACT_GLYPH_SIZE: f32 = 30.0;
const CHASE_GLYPH_SIZE: f32 = 22.0;
/// How far above the sprite's top edge the glyph's centre sits; LPC heads start a little below it.
const HEAD_CLEARANCE: f32 = 4.0;
const POP_SECONDS: f32 = 0.16;
const POP_OVERSHOOT: f32 = 1.35;
const CHASE_CUE_SECONDS: f32 = 0.9;
const CHASE_CUE_ALPHA: f32 = 0.55;
const CONTACT_COLOR: Color = Color::srgb(1.0, 0.86, 0.2);
const CHASE_COLOR: Color = Color::srgb(0.92, 0.92, 0.92);

/// UAT switches for the contact cue, read once at startup.
#[derive(Clone, Copy, Debug, PartialEq, Resource)]
pub(crate) struct ContactCueSettings {
    /// Variant B: warn when an unseen enemy starts chasing. Variant A (`false`) cues contact only.
    pub(crate) chase_warning: bool,
    /// Seconds the World holds still between the contact cue and the battle flash; 0 hands off at
    /// once, exactly as before the cue existed.
    pub(crate) beat_seconds: f32,
}

impl Default for ContactCueSettings {
    fn default() -> Self {
        Self {
            chase_warning: false,
            beat_seconds: DEFAULT_BEAT_SECONDS,
        }
    }
}

impl ContactCueSettings {
    /// No beat: contact hands off on the same frame, the way the World behaved before the cue.
    #[cfg(test)]
    pub(crate) fn immediate() -> Self {
        Self {
            beat_seconds: 0.0,
            ..Self::default()
        }
    }

    /// Resolves the switches from an environment lookup, injectable for tests the same way
    /// [`crate::world_debug_overlay::DebugCollisionOverlay::from_environment`] is. An invalid value
    /// keeps the default and logs a warning; it never fails the app.
    pub(crate) fn from_environment(environment: impl Fn(&str) -> Option<OsString>) -> Self {
        let mut settings = Self::default();
        if let Some(raw) = environment(crate::env_switches::CUE_VARIANT) {
            match parse_variant(&raw) {
                Some(chase_warning) => settings.chase_warning = chase_warning,
                None => warn!(
                    "{}={raw:?} is not A or B; using variant A",
                    crate::env_switches::CUE_VARIANT
                ),
            }
        }
        if let Some(raw) = environment(crate::env_switches::CUE_BEAT) {
            match parse_beat(&raw) {
                Some(beat) => settings.beat_seconds = beat,
                None => warn!(
                    "{}={raw:?} is not a number of seconds; using {DEFAULT_BEAT_SECONDS}",
                    crate::env_switches::CUE_BEAT
                ),
            }
        }
        settings
    }
}

fn parse_variant(raw: &OsString) -> Option<bool> {
    match raw.to_str()?.trim() {
        "A" | "a" => Some(false),
        "B" | "b" => Some(true),
        _ => None,
    }
}

/// A finite number of seconds, clamped into `0.0..=MAX_BEAT_SECONDS`.
fn parse_beat(raw: &OsString) -> Option<f32> {
    let beat = raw.to_str()?.trim().parse::<f32>().ok()?;
    beat.is_finite().then(|| beat.clamp(0.0, MAX_BEAT_SECONDS))
}

/// Written by contact detection on the frame the player touches a field enemy.
#[derive(Clone, Copy, Debug, Message)]
pub(crate) struct EnemyContactCue;

/// Written by the enemy simulation when an unseen enemy goes from not chasing to chasing.
#[derive(Clone, Copy, Debug, Message)]
pub(crate) struct UnseenChaseStarted;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EncounterCueKind {
    Contact,
    Chase,
}

/// A "!" or "?" glyph floating over the player's head.
#[derive(Component, Debug)]
pub(super) struct EncounterCueMark {
    pub(super) kind: EncounterCueKind,
    age: f32,
}

/// When the last chase warning sounded, in virtual seconds since startup.
#[derive(Debug, Default, Resource)]
pub(super) struct ChaseWarningClock {
    last: Option<f32>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "a cue reads both triggers, the font sources, and plays sound and glyph together"
)]
pub(super) fn drive_encounter_cues(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<ContactCueSettings>,
    font: CueFont,
    mut contacts: MessageReader<EnemyContactCue>,
    mut chases: MessageReader<UnseenChaseStarted>,
    mut sfx: MessageWriter<PlaySfx>,
    mut clock: ResMut<ChaseWarningClock>,
    marks: Query<(Entity, &EncounterCueMark)>,
) {
    let contacted = contacts.read().count() > 0;
    let chase_started = chases.read().count() > 0;
    if contacted {
        for (entity, _) in &marks {
            commands.entity(entity).despawn();
        }
        sfx.write(PlaySfx::new(CONTACT_STING));
        commands.spawn(cue_glyph(EncounterCueKind::Contact, font.font()));
        return;
    }
    if !(chase_started && settings.chase_warning) {
        return;
    }
    let now = time.elapsed_secs();
    if clock
        .last
        .is_some_and(|last| now - last < CHASE_WARNING_COOLDOWN_SECONDS)
    {
        return;
    }
    clock.last = Some(now);
    sfx.write(PlaySfx::boosted(
        CHASE_WARNING_SOUND,
        CHASE_WARNING_GAIN_PERCENT,
    ));
    if marks.is_empty() {
        commands.spawn(cue_glyph(EncounterCueKind::Chase, font.font()));
    }
}

fn cue_glyph(kind: EncounterCueKind, font: Handle<Font>) -> impl Bundle {
    let (glyph, size, color) = match kind {
        EncounterCueKind::Contact => ("!", CONTACT_GLYPH_SIZE, CONTACT_COLOR),
        EncounterCueKind::Chase => (
            "?",
            CHASE_GLYPH_SIZE,
            CHASE_COLOR.with_alpha(CHASE_CUE_ALPHA),
        ),
    };
    (
        Text2d::new(glyph),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        Text2dShadow {
            offset: Vec2::new(2.0, -2.0),
            color: Color::BLACK.with_alpha(if kind == EncounterCueKind::Contact {
                1.0
            } else {
                CHASE_CUE_ALPHA
            }),
        },
        Transform::from_xyz(0.0, 0.0, CUE_Z).with_scale(Vec3::ZERO),
        EncounterCueMark { kind, age: 0.0 },
    )
}

/// Pins each glyph over the player's head, pops it in, and retires the chase "?".
///
/// The contact "!" stays up through the beat and the flash; leaving the World despawns it.
pub(super) fn animate_encounter_cues(
    mut commands: Commands,
    time: Res<Time>,
    game: Option<Res<GameState>>,
    players: Query<&WorldPlayerMotion, With<WorldPlayer>>,
    mut marks: Query<(
        Entity,
        &mut EncounterCueMark,
        &mut Transform,
        &mut TextColor,
    )>,
) {
    if marks.is_empty() {
        return;
    }
    let top_left = players
        .single()
        .map(|motion| motion.top_left())
        .ok()
        .or_else(|| {
            game.as_deref()
                .map(|game| enemy_top_left(game.map().position()))
        });
    let delta = time.delta_secs();
    for (entity, mut mark, mut transform, mut color) in &mut marks {
        mark.age += delta;
        if mark.kind == EncounterCueKind::Chase && mark.age >= CHASE_CUE_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(top_left) = top_left {
            transform.translation = Vec3::new(
                top_left.x + CHARACTER_SPRITE_SIZE / 2.0,
                -top_left.y + HEAD_CLEARANCE,
                CUE_Z,
            );
        }
        transform.scale = Vec3::splat(pop_scale(mark.age));
        if mark.kind == EncounterCueKind::Chase {
            let fade = (1.0 - mark.age / CHASE_CUE_SECONDS).clamp(0.0, 1.0);
            color.0 = CHASE_COLOR.with_alpha(CHASE_CUE_ALPHA * fade.sqrt());
        }
    }
}

/// Grows past full size, then settles: a quick pop rather than a fade-in.
pub(super) fn pop_scale(age: f32) -> f32 {
    let half = POP_SECONDS / 2.0;
    if age <= 0.0 {
        0.0
    } else if age < half {
        POP_OVERSHOOT * age / half
    } else if age < POP_SECONDS {
        POP_OVERSHOOT + (1.0 - POP_OVERSHOOT) * (age - half) / half
    } else {
        1.0
    }
}

pub(super) fn cleanup_encounter_cues(
    mut commands: Commands,
    marks: Query<Entity, With<EncounterCueMark>>,
    mut clock: ResMut<ChaseWarningClock>,
) {
    for entity in &marks {
        commands.entity(entity).despawn();
    }
    *clock = ChaseWarningClock::default();
}

/// Run condition: false while the World holds still for the contact beat.
///
/// Systems that keep simulating through the battle flash (NPC wandering, which draws from
/// `GameplayRng`) use this so the beat cannot add draws: they freeze for the beat and then run
/// through the flash exactly as they did before the beat existed.
pub(crate) fn contact_beat_inactive(transition: Option<Res<BattleTransition>>) -> bool {
    transition.is_none_or(|transition| !transition.in_contact_beat())
}

/// The scenario font, as the pause overlay loads it.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct CueFont<'w> {
    asset_server: Res<'w, AssetServer>,
    root: Option<Res<'w, ScenarioRoot>>,
    inventory: Option<Res<'w, ScenarioInventory>>,
}

impl CueFont<'_> {
    fn font(&self) -> Handle<Font> {
        let (Some(root), Some(inventory)) = (&self.root, &self.inventory) else {
            return Handle::default();
        };
        inventory
            .font
            .as_ref()
            .map(|path| self.asset_server.load(root.resolve(path)))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario_spatial::Position;
    use bevy::{state::app::StatesPlugin, time::TimeUpdateStrategy};
    use std::time::Duration;

    fn settings(variant: Option<&str>, beat: Option<&str>) -> ContactCueSettings {
        ContactCueSettings::from_environment(|name| {
            if name == crate::env_switches::CUE_VARIANT {
                variant.map(OsString::from)
            } else if name == crate::env_switches::CUE_BEAT {
                beat.map(OsString::from)
            } else {
                None
            }
        })
    }

    #[test]
    fn defaults_are_variant_a_with_a_half_second_beat() {
        assert_eq!(settings(None, None), ContactCueSettings::default());
        assert!(!ContactCueSettings::default().chase_warning);
        assert_eq!(ContactCueSettings::default().beat_seconds, 0.5);
    }

    #[test]
    fn variant_switch_accepts_a_and_b_and_ignores_anything_else() {
        assert!(!settings(Some("A"), None).chase_warning);
        assert!(settings(Some("B"), None).chase_warning);
        assert!(settings(Some("b"), None).chase_warning);
        assert!(settings(Some(" B "), None).chase_warning);
        for invalid in ["", "C", "AB", "1", "variant b"] {
            assert_eq!(
                settings(Some(invalid), None),
                ContactCueSettings::default(),
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn beat_switch_clamps_and_rejects_non_numbers() {
        assert_eq!(settings(None, Some("0.3")).beat_seconds, 0.3);
        assert_eq!(settings(None, Some("0")).beat_seconds, 0.0);
        assert_eq!(settings(None, Some("1.5")).beat_seconds, 1.5);
        assert_eq!(settings(None, Some("9")).beat_seconds, MAX_BEAT_SECONDS);
        assert_eq!(settings(None, Some("-1")).beat_seconds, 0.0);
        for invalid in ["", "fast", "NaN", "inf", "0.5s"] {
            assert_eq!(
                settings(None, Some(invalid)).beat_seconds,
                DEFAULT_BEAT_SECONDS,
                "{invalid:?}"
            );
        }
        let both = settings(Some("B"), Some("0.8"));
        assert!(both.chase_warning);
        assert_eq!(both.beat_seconds, 0.8);
    }

    fn beat_inactive(world: &mut World) -> bool {
        let mut condition = IntoSystem::into_system(contact_beat_inactive);
        condition.initialize(world);
        condition
            .run((), world)
            .expect("the run condition only reads an optional resource")
    }

    #[test]
    fn rng_drawing_world_systems_hold_only_for_the_beat() {
        let mut world = World::new();
        assert!(beat_inactive(&mut world), "no transition resource");
        world.insert_resource(BattleTransition::default());
        assert!(beat_inactive(&mut world), "idle");
        world
            .resource_mut::<BattleTransition>()
            .request_after_beat(0.5);
        assert!(!beat_inactive(&mut world), "contact beat");
        *world.resource_mut::<BattleTransition>() = BattleTransition::default();
        world.resource_mut::<BattleTransition>().request();
        assert!(beat_inactive(&mut world), "the flash runs as it always has");
    }

    #[test]
    fn pop_grows_past_full_size_then_settles() {
        assert_eq!(pop_scale(0.0), 0.0);
        assert!(pop_scale(POP_SECONDS / 2.0) > 1.2);
        assert_eq!(pop_scale(POP_SECONDS), 1.0);
        assert_eq!(pop_scale(5.0), 1.0);
    }

    /// Only the cue systems, driven by hand-written trigger messages.
    fn cue_app(chase_warning: bool) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_asset::<Font>()
            .add_message::<PlaySfx>()
            .add_message::<EnemyContactCue>()
            .add_message::<UnseenChaseStarted>()
            .init_resource::<ChaseWarningClock>()
            .insert_resource(ContactCueSettings {
                chase_warning,
                beat_seconds: 0.5,
            })
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                100,
            )))
            .add_systems(
                Update,
                (drive_encounter_cues, animate_encounter_cues).chain(),
            );
        app.world_mut().spawn((
            WorldPlayer,
            WorldPlayerMotion::from_tile(Position::new(4, 4)),
        ));
        app.update();
        app
    }

    fn sounds(app: &App) -> Vec<PlaySfx> {
        app.world()
            .resource::<Messages<PlaySfx>>()
            .iter_current_update_messages()
            .copied()
            .collect()
    }

    fn marks(app: &mut App) -> Vec<(EncounterCueKind, Vec3)> {
        let world = app.world_mut();
        world
            .query::<(&EncounterCueMark, &Transform)>()
            .iter(world)
            .map(|(mark, transform)| (mark.kind, transform.translation))
            .collect()
    }

    #[test]
    fn contact_pops_a_bang_over_the_player_and_plays_the_sting_that_frame() {
        let mut app = cue_app(false);
        app.world_mut().write_message(EnemyContactCue);
        app.update();
        assert_eq!(sounds(&app), [PlaySfx::new(CONTACT_STING)]);
        let marks = marks(&mut app);
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].0, EncounterCueKind::Contact);
        let top_left = WorldPlayerMotion::from_tile(Position::new(4, 4)).top_left();
        assert_eq!(marks[0].1.x, top_left.x + CHARACTER_SPRITE_SIZE / 2.0);
        assert!(marks[0].1.y > -top_left.y, "the glyph sits above the head");
        // It stays up for the beat and the flash.
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(self::marks(&mut app).len(), 1);
    }

    #[test]
    fn variant_b_warns_once_per_chase_start_and_rate_limits_a_crowd() {
        let mut app = cue_app(true);
        // Three enemies start chasing on one frame: one warning.
        for _ in 0..3 {
            app.world_mut().write_message(UnseenChaseStarted);
        }
        app.update();
        assert_eq!(
            sounds(&app),
            [PlaySfx::boosted(
                CHASE_WARNING_SOUND,
                CHASE_WARNING_GAIN_PERCENT
            )]
        );
        assert_eq!(marks(&mut app)[0].0, EncounterCueKind::Chase);
        // A continuing chase writes nothing, and so warns nothing.
        app.update();
        assert!(sounds(&app).is_empty());
        // Another start inside the cooldown stays quiet.
        app.world_mut().write_message(UnseenChaseStarted);
        app.update();
        assert!(sounds(&app).is_empty());
        // The faint "?" retires on its own.
        for _ in 0..10 {
            app.update();
        }
        assert!(marks(&mut app).is_empty());
        // After the cooldown a new start warns again.
        for _ in 0..10 {
            app.update();
        }
        app.world_mut().write_message(UnseenChaseStarted);
        app.update();
        assert_eq!(sounds(&app).len(), 1);
    }

    #[test]
    fn variant_a_never_warns_on_a_chase_start() {
        let mut app = cue_app(false);
        app.world_mut().write_message(UnseenChaseStarted);
        app.update();
        assert!(sounds(&app).is_empty());
        assert!(marks(&mut app).is_empty());
    }

    #[test]
    fn contact_replaces_a_pending_chase_question_mark() {
        let mut app = cue_app(true);
        app.world_mut().write_message(UnseenChaseStarted);
        app.update();
        app.world_mut().write_message(EnemyContactCue);
        app.update();
        let marks = marks(&mut app);
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].0, EncounterCueKind::Contact);
    }
}
