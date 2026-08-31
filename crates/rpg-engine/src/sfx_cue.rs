//! One-shot scenario sound effects, addressed by logical key.
//!
//! Every caller that wants a sound writes [`PlaySfx::new("heal")`](PlaySfx) and stops there. This
//! module owns the parts that are otherwise copied into every call site: holding the
//! `Handle<SfxIndex>`, waiting for it to load, resolving a logical key to a package-relative asset
//! path, and spawning the despawn-on-finish player entity.
//!
//! That indirection is the point. Before it, playing a sound meant a system also had to take
//! `Res<AssetServer>`, `Res<ScenarioRoot>` and `Res<Assets<SfxIndex>>`, keep its own handle alive,
//! and handle the not-yet-loaded case — enough friction that 23 authored cues shipped with 3 of
//! them reachable (roadmap A-1). A message keeps the cost of adding a cue at one line.
//!
//! Muting is not handled here: `RPG_S1_MUTE_AUDIO` sets the global volume to zero when the app is
//! built, which covers every player uniformly.

use bevy::{ecs::system::SystemParam, prelude::*};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    scenario_audio::{SFX_INDEX_PATH, SfxIndex},
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
};

/// A request to play one authored SFX, by its `sfx_index.yaml` key.
///
/// Keys are `&'static str` so a typo is a compile-time constant in one place rather than a string
/// built at a call site; the shared names live in [`cue`].
#[derive(Clone, Copy, Debug, Eq, Message, PartialEq)]
pub(crate) struct PlaySfx {
    key: &'static str,
}

impl PlaySfx {
    pub(crate) const fn new(key: &'static str) -> Self {
        Self { key }
    }

    pub(crate) const fn key(self) -> &'static str {
        self.key
    }
}

/// The logical cue keys authored in `data/audio/sfx_index.yaml`.
///
/// Named constants rather than bare literals so the set is greppable and a rename in the scenario
/// index has exactly one place to land.
pub(crate) mod cue {
    pub(crate) const HOVER: &str = "hover";
    pub(crate) const CONFIRM: &str = "confirm";
    pub(crate) const CANCEL: &str = "cancel";
    pub(crate) const DENIED: &str = "denied";
    pub(crate) const USE_ITEM: &str = "use_item";

    pub(crate) const ATK_SLASH: &str = "atk_slash";
    pub(crate) const ATK_IMPACT: &str = "atk_impact";
    pub(crate) const PARTY_HIT: &str = "party_hit";
    pub(crate) const DEFEND: &str = "defend";
    pub(crate) const FLEE: &str = "flee";
    pub(crate) const ENEMY_DEATH: &str = "enemy_death";
    pub(crate) const HEAL: &str = "heal";
    pub(crate) const REVIVE: &str = "revive";
    pub(crate) const ATK_BUFF: &str = "atk_buff";
    pub(crate) const DEF_BUFF: &str = "def_buff";
    pub(crate) const DEBUFF: &str = "debuff";

    /// Element spell cues, `spell_<element>`; see `ability_spell_cue` in `battle::fx`.
    ///
    /// `sfx_index.yaml` also authors `spell_ice` and `spell_thunder`. They are deliberately absent
    /// here: neither `AbilityElement` nor `ItemElement` has an Ice or Thunder variant, so no game
    /// action can reach them. They are unreachable inherited content, not a routing gap.
    pub(crate) const SPELL_FIRE: &str = "spell_fire";
    pub(crate) const SPELL_WATER: &str = "spell_water";
    pub(crate) const SPELL_WIND: &str = "spell_wind";
    pub(crate) const SPELL_EARTH: &str = "spell_earth";

    /// A whiff. The pinned engine plays nothing here and lets the floating MISS label carry the
    /// beat on its own; giving it a cue is a deliberate widening, not a parity fix.
    pub(crate) const MISS: &str = "miss";

    /// Afflictions distinctive enough not to share the generic [`DEBUFF`] cue with every other
    /// status. Sleep in particular is now reachable content — see `battle::enemy_ai`.
    pub(crate) const STATUS_SLEEP: &str = "status_sleep";
    pub(crate) const STATUS_POISON: &str = "status_poison";
    pub(crate) const SPEED_BUFF: &str = "speed_buff";

    /// What an enemy's basic attack sounds like, chosen by its authored type. Types with no
    /// distinctive sample keep [`ATK_IMPACT`]; forcing one on them would be worse than sharing.
    pub(crate) const ATK_CLAW: &str = "atk_claw";
    pub(crate) const ATK_SWORD: &str = "atk_sword";

    /// The struck enemy's own reaction. Striking one used to play the party's weapon and nothing
    /// back, while being struck played both the impact and the party's hurt — this restores the
    /// symmetry.
    pub(crate) const ENEMY_HIT: &str = "enemy_hit";

    /// Field-object and transition cues.
    pub(crate) const CHEST_OPEN: &str = "chest_open";
    pub(crate) const CHEST_CLOSE: &str = "chest_close";
    pub(crate) const DOOR: &str = "door";
    pub(crate) const TELEPORT: &str = "teleport";

    /// Equipment and shop beats, which previously reused the plain confirm cue.
    pub(crate) const EQUIP: &str = "equip";
    pub(crate) const UNEQUIP: &str = "unequip";
    pub(crate) const BUY_SELL: &str = "buy_sell";

    /// Opening and closing the field menu, which is what stops and restarts the world.
    pub(crate) const PAUSE: &str = "pause";
    pub(crate) const UNPAUSE: &str = "unpause";

    /// Cues that rotate through several authored samples instead of repeating one.
    ///
    /// Callers always write the base key — the first entry of a group — and the cue service
    /// substitutes the next sample in rotation. The point is the beats a player hears hundreds of
    /// times in an evening: every sword landing on the identical waveform is the thing that makes
    /// a battle sound cheap.
    ///
    /// Rotation is a counter, not a random draw. Audio must not touch `GameplayRng`, because the
    /// `record`/`replay` commands depend on that stream being consumed identically; a per-cue
    /// cursor is varied to the ear and still perfectly reproducible.
    pub(crate) const VARIANT_GROUPS: &[&[&str]] = &[
        &[ATK_IMPACT, "atk_impact_2", "atk_impact_3", "atk_impact_4"],
        &[PARTY_HIT, "party_hit_2", "party_hit_3", "party_hit_4"],
        &[MISS, "miss_2", "miss_3", "miss_4"],
        &[ATK_CLAW, "atk_claw_2"],
        &[
            ATK_SWORD,
            "atk_sword_2",
            "atk_sword_3",
            "atk_sword_4",
            "atk_sword_5",
            "atk_sword_6",
        ],
        &[ENEMY_HIT, "enemy_hit_2", "enemy_hit_3"],
        &[CHEST_OPEN, "chest_open_2", "chest_open_3", "chest_open_4"],
        &[CHEST_CLOSE, "chest_close_2", "chest_close_3"],
        &[DOOR, "door_2"],
    ];

    /// The rotation group a cue belongs to, if it has one.
    pub(crate) fn variants_of(key: &str) -> Option<&'static [&'static str]> {
        VARIANT_GROUPS.iter().copied().find(|group| group[0] == key)
    }
}

/// The four sounds every menu screen makes, as one injectable parameter.
///
/// This is the port of the pinned engine's menu-SFX mixin
/// (`engine/common/menu_sfx_mixin.py:18-30`), which every menu scene there inherits so that
/// navigation sounds the same everywhere. A `SystemParam` is the Bevy equivalent: a screen adds
/// `menu_sfx: MenuSfx` to its input system and calls the beat it just performed.
///
/// Keeping [`blocked`](Self::blocked) in the same place as the rest is deliberate. A menu that
/// refuses a move needs to say so, and a screen that has the other three to hand is far more
/// likely to add the fourth.
#[derive(SystemParam)]
pub(crate) struct MenuSfx<'w> {
    writer: MessageWriter<'w, PlaySfx>,
}

impl MenuSfx<'_> {
    /// The selection moved to a different entry.
    pub(crate) fn hover(&mut self) {
        self.writer.write(PlaySfx::new(cue::HOVER));
    }

    /// An entry was accepted.
    pub(crate) fn confirm(&mut self) {
        self.writer.write(PlaySfx::new(cue::CONFIRM));
    }

    /// The screen or a submenu was backed out of.
    pub(crate) fn cancel(&mut self) {
        self.writer.write(PlaySfx::new(cue::CANCEL));
    }

    /// The action was refused — unaffordable, locked, or otherwise unavailable.
    pub(crate) fn blocked(&mut self) {
        self.writer.write(PlaySfx::new(cue::DENIED));
    }

    /// An arbitrary cue, for screens that need one outside the four menu beats.
    pub(crate) fn play(&mut self, key: &'static str) {
        self.writer.write(PlaySfx::new(key));
    }
}

/// Holds the scenario SFX index alive for the lifetime of the app.
#[derive(Resource, Default)]
pub(crate) struct SfxCatalog {
    index: Option<Handle<SfxIndex>>,
    /// How many times each rotating cue has sounded, so its variants cycle in order.
    rotations: BTreeMap<&'static str, usize>,
}

impl SfxCatalog {
    /// Resolves a requested cue to the sample that should sound now, advancing its rotation.
    ///
    /// A cue with no authored variants returns itself and costs nothing.
    fn next_sample(&mut self, key: &'static str) -> &'static str {
        let Some(group) = cue::variants_of(key) else {
            return key;
        };
        let turn = self.rotations.entry(key).or_default();
        let sample = group[*turn % group.len()];
        *turn = turn.wrapping_add(1);
        sample
    }
}

pub(crate) struct SfxCuePlugin;

impl Plugin for SfxCuePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .init_resource::<SfxCatalog>()
            .add_systems(Update, play_requested_cues);
    }
}

/// Drains this frame's cue requests and spawns one player per distinct key.
///
/// De-duplicating by key matters: a single battle action can emit several events that map to the
/// same cue (an area spell resolving against four enemies is four `MagicDamage` events), and
/// spawning four copies of one sample in a frame is audibly wrong — it is one cast, and it phases
/// against itself.
///
/// Requests are dropped rather than queued while the index is still loading. These are one-shots
/// tied to a moment that has already passed by the time the asset arrives, and audio never fails
/// the app.
fn play_requested_cues(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    indexes: Res<Assets<SfxIndex>>,
    mut catalog: ResMut<SfxCatalog>,
    mut requests: MessageReader<PlaySfx>,
) {
    let requested = requests
        .read()
        .map(|request| request.key())
        .collect::<BTreeSet<_>>();
    if requested.is_empty() {
        return;
    }

    let handle = catalog.index.get_or_insert_with(|| {
        let path = ScenarioRelativePath::try_from(SFX_INDEX_PATH)
            .expect("the SFX index path is scenario-relative");
        asset_server.load(root.resolve(&path))
    });
    let Some(index) = indexes.get(&*handle) else {
        return;
    };

    for key in requested {
        let sample = catalog.next_sample(key);
        let Some(path) = index.resolve_key(&root, sample) else {
            continue;
        };
        commands.spawn((
            AudioPlayer::new(asset_server.load(path)),
            PlaybackSettings::DESPAWN,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;
    use std::time::Duration;

    /// Primes the lazy index load and pumps until it resolves, so assertions see a live catalog.
    /// The priming cue is consumed while the index is still loading, so it spawns nothing.
    fn ready_sfx_app() -> App {
        let mut app = test_support::headless_sfx_app();
        app.world_mut().write_message(PlaySfx::new(cue::HOVER));
        for _ in 0..5_000 {
            app.update();
            let loaded = {
                let catalog = app.world().resource::<SfxCatalog>();
                let indexes = app.world().resource::<Assets<SfxIndex>>();
                catalog
                    .index
                    .as_ref()
                    .is_some_and(|handle| indexes.get(handle).is_some())
            };
            if loaded {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            player_paths(&mut app),
            Vec::<String>::new(),
            "priming must not leave a player behind"
        );
        app
    }

    fn player_paths(app: &mut App) -> Vec<String> {
        let world = app.world_mut();
        let mut players = world.query::<&AudioPlayer<AudioSource>>();
        let handles = players
            .iter(world)
            .map(|player| player.0.id())
            .collect::<Vec<_>>();
        let server = world.resource::<AssetServer>();
        let mut paths = handles
            .into_iter()
            .filter_map(|id| server.get_path(id))
            .map(|path| path.path().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    /// Two cues in one frame play once each; a repeated cue plays once, not twice.
    #[test]
    fn distinct_cues_play_once_each_and_repeats_collapse() {
        let mut app = ready_sfx_app();
        app.world_mut().write_message(PlaySfx::new(cue::HOVER));
        app.world_mut().write_message(PlaySfx::new(cue::HOVER));
        app.world_mut().write_message(PlaySfx::new(cue::CONFIRM));
        app.update();

        assert_eq!(
            player_paths(&mut app),
            vec![
                "scenarios/minimal_demo/assets/audio/sfx/confirm.mp3".to_owned(),
                "scenarios/minimal_demo/assets/audio/sfx/hover.mp3".to_owned(),
            ]
        );
    }

    /// An unauthored key is ignored rather than panicking or spawning a silent player.
    #[test]
    fn an_unknown_cue_key_is_dropped() {
        let mut app = ready_sfx_app();
        app.world_mut()
            .write_message(PlaySfx::new("no_such_cue_key"));
        app.update();

        assert_eq!(player_paths(&mut app), Vec::<String>::new());
    }

    /// Every cue name this crate can emit must exist in the scenario that actually ships.
    ///
    /// The keys are plain strings by the time they reach the index, so nothing else would catch a
    /// typo or a rename on the content side — the sound would simply never play, which is the
    /// failure mode this whole module exists to end.
    #[test]
    fn every_cue_constant_resolves_against_the_shipped_index() {
        let index: SfxIndex = crate::scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/audio/sfx_index.yaml"
        ))
        .expect("the shipped SFX index parses");

        for key in [
            cue::HOVER,
            cue::CONFIRM,
            cue::CANCEL,
            cue::DENIED,
            cue::USE_ITEM,
            cue::ATK_SLASH,
            cue::ATK_IMPACT,
            cue::PARTY_HIT,
            cue::DEFEND,
            cue::FLEE,
            cue::ENEMY_DEATH,
            cue::HEAL,
            cue::REVIVE,
            cue::ATK_BUFF,
            cue::DEF_BUFF,
            cue::DEBUFF,
            cue::SPELL_FIRE,
            cue::SPELL_WATER,
            cue::SPELL_WIND,
            cue::SPELL_EARTH,
            cue::MISS,
            cue::STATUS_SLEEP,
            cue::STATUS_POISON,
            cue::SPEED_BUFF,
            cue::ATK_CLAW,
            cue::ATK_SWORD,
            cue::CHEST_OPEN,
            cue::CHEST_CLOSE,
            cue::DOOR,
            cue::TELEPORT,
            cue::EQUIP,
            cue::UNEQUIP,
            cue::BUY_SELL,
            cue::PAUSE,
            cue::UNPAUSE,
        ] {
            assert!(
                index.path_for_key(key).is_some(),
                "cue `{key}` is not authored in the shipped sfx_index.yaml"
            );
        }

        // Every rotation variant has to resolve too, or a cue would fall silent partway
        // through its cycle instead of failing loudly.
        for group in cue::VARIANT_GROUPS {
            for key in *group {
                assert!(
                    index.path_for_key(key).is_some(),
                    "variant `{key}` is not authored in the shipped sfx_index.yaml"
                );
            }
            let mut paths = group
                .iter()
                .map(|key| index.path_for_key(key).unwrap().as_str())
                .collect::<Vec<_>>();
            let authored = paths.len();
            paths.sort_unstable();
            paths.dedup();
            assert_eq!(
                paths.len(),
                authored,
                "rotation group `{}` points two variants at one sample",
                group[0]
            );
        }
    }

    /// A rotating cue walks its group in order and wraps; a plain cue always returns itself.
    #[test]
    fn rotating_cues_cycle_their_variants_deterministically() {
        let mut catalog = SfxCatalog::default();
        let group = cue::variants_of(cue::CHEST_OPEN).expect("chest_open rotates");
        let heard = (0..group.len() * 2)
            .map(|_| catalog.next_sample(cue::CHEST_OPEN))
            .collect::<Vec<_>>();
        assert_eq!(heard[..group.len()], *group);
        assert_eq!(heard[group.len()..], *group, "the rotation wraps");

        assert!(cue::variants_of(cue::HOVER).is_none());
        assert_eq!(catalog.next_sample(cue::HOVER), cue::HOVER);
        assert_eq!(catalog.next_sample(cue::HOVER), cue::HOVER);
    }

    /// Groups are addressed by their first entry, which is the key callers actually write.
    #[test]
    fn every_rotation_group_is_named_by_a_real_base_cue() {
        for group in cue::VARIANT_GROUPS {
            assert!(group.len() > 1, "a one-sample group is not a rotation");
            assert_eq!(cue::variants_of(group[0]), Some(*group));
        }
    }

    /// No request means no work: the index is not loaded until something asks for a sound.
    #[test]
    fn the_index_is_not_loaded_until_a_cue_is_requested() {
        let mut app = test_support::headless_sfx_app();
        app.update();
        assert!(app.world().resource::<SfxCatalog>().index.is_none());

        app.world_mut().write_message(PlaySfx::new(cue::HOVER));
        app.update();
        assert!(app.world().resource::<SfxCatalog>().index.is_some());
    }
}
