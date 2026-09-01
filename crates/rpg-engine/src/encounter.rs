//! Deterministic encounter selection and battle-entry domain state.

use std::{collections::BTreeMap, error::Error, fmt};

use bevy::prelude::Resource;

use crate::{
    field_menu_domain::{FieldMenuCatalog, derived_stats},
    gameplay_rng::GameplayRng,
    runtime_map::RuntimeMapId,
    runtime_member::EquipmentSlot,
    runtime_party::RuntimeParty,
    runtime_repository::RuntimeRepository,
    scenario_balance::SpawnerBalance,
    scenario_encounter::{EncounterFormation, EncounterZone},
    scenario_enemy::{BossMoveSet, EnemyBehavior, EnemyDefinition, EnemyDrops, EnemySize},
    scenario_party::PartyRow,
    scenario_spatial::{CardinalDirection, Position},
};

/// Map-keyed encounter data. Maps absent from this catalog are explicitly encounter-free.
#[derive(Clone, Debug, Default, PartialEq, Resource)]
pub struct EncounterCatalog(BTreeMap<String, EncounterZone>);

impl EncounterCatalog {
    pub fn try_from_zones(
        zones: impl IntoIterator<Item = (String, EncounterZone)>,
    ) -> Result<Self, EncounterCatalogError> {
        let mut catalog = BTreeMap::new();
        for (filename_stem, zone) in zones {
            let id = zone.effective_id(&filename_stem).to_owned();
            if catalog.insert(id.clone(), zone).is_some() {
                return Err(EncounterCatalogError::DuplicateZoneId(id));
            }
        }
        Ok(Self(catalog))
    }

    pub fn zone_for_map(&self, map_id: &str) -> Option<&EncounterZone> {
        self.0.get(map_id)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncounterCatalogError {
    DuplicateZoneId(String),
}

impl fmt::Display for EncounterCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateZoneId(id) => write!(formatter, "duplicate encounter-zone id `{id}`"),
        }
    }
}

impl Error for EncounterCatalogError {}

/// Indexed immutable enemy definitions loaded from the source rank streams.
#[derive(Clone, Debug, Default, PartialEq, Resource)]
pub struct EnemyCatalog(BTreeMap<String, EnemyDefinition>);

impl EnemyCatalog {
    pub fn try_from_definitions(
        definitions: impl IntoIterator<Item = EnemyDefinition>,
    ) -> Result<Self, EnemyCatalogError> {
        let mut catalog = BTreeMap::new();
        for definition in definitions {
            let id = definition.id.clone();
            if catalog.insert(id.clone(), definition).is_some() {
                return Err(EnemyCatalogError::DuplicateEnemyId(id));
            }
        }
        Ok(Self(catalog))
    }

    pub fn enemy(&self, id: &str) -> Option<&EnemyDefinition> {
        self.0.get(id)
    }

    pub fn resolve_boss_move_sets<'a>(
        &mut self,
        move_sets: impl IntoIterator<Item = (&'a str, &'a BossMoveSet)>,
    ) -> Result<(), EnemyCatalogError> {
        let move_sets = move_sets.into_iter().collect::<BTreeMap<_, _>>();
        for enemy in self.0.values_mut() {
            let EnemyBehavior::Referenced { ai_ref } = &enemy.behavior else {
                continue;
            };
            let path = ai_ref.as_str().to_owned();
            let move_set = move_sets.get(path.as_str()).ok_or_else(|| {
                EnemyCatalogError::MissingBossMoveSet {
                    enemy_id: enemy.id.clone(),
                    path: path.clone(),
                }
            })?;
            enemy.behavior = EnemyBehavior::Inline {
                ai: move_set.ai.clone(),
                targeting: move_set.targeting.clone(),
            };
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyCatalogError {
    DuplicateEnemyId(String),
    MissingBossMoveSet { enemy_id: String, path: String },
}

impl fmt::Display for EnemyCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateEnemyId(id) => write!(formatter, "duplicate enemy id `{id}`"),
            Self::MissingBossMoveSet { enemy_id, path } => write!(
                formatter,
                "enemy `{enemy_id}` references unloaded boss move set `{path}`"
            ),
        }
    }
}

impl Error for EnemyCatalogError {}

/// Selects one positive-weight formation without allocation or floating-point drift.
pub fn pick_weighted_formation<'a>(
    entries: &'a [EncounterFormation],
    rng: &mut GameplayRng,
) -> Option<&'a EncounterFormation> {
    let total = entries
        .iter()
        .map(|entry| u64::from(entry.weight))
        .sum::<u64>();
    if total == 0 {
        return None;
    }
    pick_weighted_formation_at(entries, rng.next_u64() % total)
}

fn pick_weighted_formation_at(
    entries: &[EncounterFormation],
    mut roll: u64,
) -> Option<&EncounterFormation> {
    for entry in entries {
        let weight = u64::from(entry.weight);
        if roll < weight {
            return Some(entry);
        }
        roll = roll.saturating_sub(weight);
    }
    None
}

/// Effective visible-enemy cadence and chase adjustments for the active party.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncounterModifiers {
    pub interval_multiplier: f32,
    pub chase_range_reduction: u32,
}

impl Default for EncounterModifiers {
    fn default() -> Self {
        Self {
            interval_multiplier: 1.0,
            chase_range_reduction: 0,
        }
    }
}

pub fn party_encounter_modifiers(
    party: &RuntimeParty,
    balance: &SpawnerBalance,
) -> EncounterModifiers {
    party_encounter_modifiers_for(
        party.members().map(|member| {
            (
                member.class_id(),
                member.equipment().get(EquipmentSlot::Accessory),
            )
        }),
        balance,
    )
}

fn party_encounter_modifiers_for<'a>(
    members: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
    balance: &SpawnerBalance,
) -> EncounterModifiers {
    let mut modifiers = EncounterModifiers::default();
    let mut has_rogue = false;
    for (_, accessory) in members
        .into_iter()
        .filter(|(class_id, _)| *class_id == "rogue")
    {
        has_rogue = true;
        modifiers.chase_range_reduction = modifiers
            .chase_range_reduction
            .saturating_add(balance.rogue_chase_reduction);
        match accessory {
            Some("stealth_cloak") => {
                modifiers.chase_range_reduction = modifiers
                    .chase_range_reduction
                    .saturating_add(balance.stealth_cloak_reduction);
            }
            Some("lure_charm") => {
                modifiers.interval_multiplier = modifiers
                    .interval_multiplier
                    .min(balance.lure_charm_interval_mult.get() as f32);
            }
            _ => {}
        }
    }
    if has_rogue {
        // Pinned Python behavior: the party-wide Rogue passive reduces encounter cadence by 20%.
        modifiers.interval_multiplier /= 0.8;
    }
    modifiers
}

/// Timer for reactivating a visible enemy. It never emits a battle request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpawnCadence {
    elapsed: f32,
    interval: f32,
}

impl SpawnCadence {
    pub fn new(interval: f32) -> Self {
        Self {
            elapsed: 0.0,
            interval: interval.max(f32::EPSILON),
        }
    }

    pub fn advance(&mut self, seconds: f32, multiplier: f32, has_inactive: bool) -> bool {
        if !has_inactive {
            self.elapsed = 0.0;
            return false;
        }
        self.elapsed += seconds.max(0.0);
        let effective = self.interval * multiplier.max(f32::EPSILON);
        if self.elapsed < effective {
            return false;
        }
        self.elapsed = 0.0;
        true
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BattleSide {
    Party,
    Enemy,
}

/// Initial mutable battle values copied at the encounter boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct BattleParticipant {
    pub side: BattleSide,
    pub id: String,
    pub name: String,
    pub class_id: String,
    pub health: u32,
    pub max_health: u32,
    pub mana: u32,
    pub max_mana: u32,
    pub attack: i64,
    pub defense: i64,
    pub magic_resistance: i64,
    pub dexterity: i64,
    pub abilities: Vec<crate::scenario_class::Ability>,
    pub status_effects: Vec<crate::scenario_item::ItemStatus>,
    pub accessory: Option<String>,
    pub row: PartyRow,
    pub boss: bool,
    pub enemy_type: Option<crate::scenario_enemy::EnemyType>,
    pub immunities: Vec<crate::scenario_enemy::EnemyImmunity>,
    pub behavior: Option<crate::scenario_enemy::EnemyBehavior>,
    pub experience_yield: u32,
    pub enemy_size: Option<EnemySize>,
    pub sprite_id: String,
    pub sprite_scale_percent: u32,
    pub drops: Option<EnemyDrops>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreBattleReturnContext {
    pub map_id: String,
    pub position: Position,
    pub facing: CardinalDirection,
    pub world_bgm_key: Option<String>,
    pub world_enemies: Vec<WorldEnemyReturnState>,
}

/// Recoverable visible-enemy pool captured after engagement and before World cleanup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldEnemyReturnState {
    pub encounter_id: String,
    pub formation: Vec<String>,
    pub origin: Position,
    pub position: Position,
    pub facing: CardinalDirection,
    pub boss: bool,
    pub chase_range: u32,
    pub active: bool,
}

pub(crate) fn restore_pre_battle_context(
    game: &mut crate::game_state::GameState,
    context: &PreBattleReturnContext,
) -> Result<(), crate::runtime_map::RuntimeMapIdError> {
    let map_id = RuntimeMapId::try_new(context.map_id.clone())?;
    if game.map().current() == Some(&map_id) {
        game.map_mut().set_position(context.position);
        game.map_mut().set_facing(context.facing);
    } else {
        game.map_mut()
            .move_to(map_id, context.position, context.facing);
    }
    Ok(())
}

/// Complete M8 handoff to the later battle loop.
#[derive(Clone, Debug, PartialEq, Resource)]
pub struct BattleEntry {
    pub encounter_id: String,
    pub participants: Vec<BattleParticipant>,
    pub background_id: String,
    pub background_asset: String,
    pub bgm_key: String,
    pub boss_completion_flag: Option<String>,
    pub barrier_messages: Vec<String>,
    pub return_context: PreBattleReturnContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildBattleError {
    UnknownEnemy(String),
    EmptyFormation,
    /// A `start_battle` named an enemy the zone gates behind a key item. A scripted duel has no
    /// second enemy to fall back to, so this would build an empty formation.
    BarrieredScriptedEnemy(String),
}

impl fmt::Display for BuildBattleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEnemy(id) => {
                write!(formatter, "encounter references unknown enemy `{id}`")
            }
            Self::EmptyFormation => formatter.write_str("encounter produced no eligible enemies"),
            Self::BarrieredScriptedEnemy(id) => write!(
                formatter,
                "scripted battle enemy `{id}` is a barrier enemy of this zone"
            ),
        }
    }
}

impl Error for BuildBattleError {}

#[expect(
    clippy::too_many_arguments,
    reason = "battle entry copies each independently owned encounter boundary"
)]
pub(crate) fn build_battle_entry(
    encounter_id: &str,
    formation: &[String],
    zone: &EncounterZone,
    enemy_catalog: &EnemyCatalog,
    item_catalog: &FieldMenuCatalog,
    party: &RuntimeParty,
    repository: &RuntimeRepository,
    flags: &crate::runtime_flags::RuntimeFlags,
    boss: bool,
    return_context: PreBattleReturnContext,
) -> Result<BattleEntry, BuildBattleError> {
    let mut participants = party
        .members()
        .map(|member| {
            let stats = derived_stats(member, item_catalog);
            BattleParticipant {
                side: BattleSide::Party,
                id: member.id().to_owned(),
                name: member.name().to_owned(),
                class_id: member.class_id().to_owned(),
                health: member.health(),
                max_health: member.max_health(),
                mana: member.mana(),
                max_mana: member.max_mana(),
                attack: i64::from(stats.strength),
                defense: i64::from(stats.constitution),
                magic_resistance: i64::from(stats.intelligence),
                dexterity: i64::from(stats.dexterity),
                abilities: item_catalog
                    .unlocked_abilities(member.class_id(), member.level(), flags)
                    .into_iter()
                    .cloned()
                    .collect(),
                status_effects: member.status_effects().collect(),
                accessory: member
                    .equipment()
                    .get(EquipmentSlot::Accessory)
                    .map(str::to_owned),
                row: member.row(),
                boss: false,
                enemy_type: None,
                immunities: Vec::new(),
                behavior: None,
                experience_yield: 0,
                enemy_size: None,
                sprite_id: String::new(),
                sprite_scale_percent: 100,
                drops: None,
            }
        })
        .collect::<Vec<_>>();

    let barriers = zone
        .barrier_enemies
        .iter()
        .map(|barrier| (barrier.enemy_id.as_str(), barrier))
        .collect::<BTreeMap<_, _>>();
    let mut barrier_messages = Vec::new();
    let mut enemy_count = 0;
    for id in formation {
        if let Some(barrier) = barriers.get(id.as_str())
            && !repository.contains_item(&barrier.requires_item)
        {
            barrier_messages.push(barrier.blocked_message.clone());
            continue;
        }
        let enemy = enemy_catalog
            .enemy(id)
            .ok_or_else(|| BuildBattleError::UnknownEnemy(id.clone()))?;
        participants.push(enemy_participant(enemy));
        enemy_count += 1;
    }
    if enemy_count == 0 {
        return Err(BuildBattleError::EmptyFormation);
    }

    Ok(BattleEntry {
        encounter_id: encounter_id.to_owned(),
        participants,
        background_id: zone.background.clone(),
        background_asset: format!("assets/images/battle_bg/{}.webp", zone.background),
        bgm_key: if boss { "battle.boss" } else { "battle.normal" }.to_owned(),
        boss_completion_flag: boss
            .then_some(zone.boss.as_ref())
            .flatten()
            .map(|boss| boss.completion.set_flag.clone())
            .filter(|flag| !flag.is_empty()),
        barrier_messages,
        return_context,
    })
}

/// Builds the one-enemy battle a dialogue asks for with `on_complete.start_battle`.
///
/// Framing differences from a wandering encounter, all of them deliberate:
///
/// - The formation is exactly the named enemy, so `barrier_enemies` cannot apply — a barrier is
///   the world's way of refusing a *random* fight the player has no key for, and an authored duel
///   is not that. A barriered id would silently produce an empty formation, so it is rejected.
/// - `boss` comes from the enemy's own definition, which is what selects the boss battle theme.
/// - `boss_completion_flag` is cleared even for a boss-flagged enemy: the zone's completion flag
///   belongs to the zone's own boss encounter, and the dialogue that started this fight already
///   owns whatever story flags it means to set.
///
/// The zone is still the source of the battle background, so a map hosting a scripted battle
/// needs its `data/encount/<map_id>.yaml` — which may declare `density: 0.0` and no `entries`.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors build_battle_entry, which reads every catalog a battle is assembled from"
)]
pub(crate) fn build_scripted_battle_entry(
    enemy_id: &str,
    zone: &EncounterZone,
    enemy_catalog: &EnemyCatalog,
    item_catalog: &FieldMenuCatalog,
    party: &RuntimeParty,
    repository: &RuntimeRepository,
    flags: &crate::runtime_flags::RuntimeFlags,
    return_context: PreBattleReturnContext,
) -> Result<BattleEntry, BuildBattleError> {
    let enemy = enemy_catalog
        .enemy(enemy_id)
        .ok_or_else(|| BuildBattleError::UnknownEnemy(enemy_id.to_owned()))?;
    if zone
        .barrier_enemies
        .iter()
        .any(|barrier| barrier.enemy_id == enemy_id)
    {
        return Err(BuildBattleError::BarrieredScriptedEnemy(
            enemy_id.to_owned(),
        ));
    }
    let boss = enemy.boss;
    let formation = [enemy_id.to_owned()];
    let mut entry = build_battle_entry(
        &format!("scripted:{enemy_id}"),
        &formation,
        zone,
        enemy_catalog,
        item_catalog,
        party,
        repository,
        flags,
        boss,
        return_context,
    )?;
    entry.boss_completion_flag = None;
    Ok(entry)
}

fn enemy_participant(enemy: &EnemyDefinition) -> BattleParticipant {
    BattleParticipant {
        side: BattleSide::Enemy,
        id: enemy.id.clone(),
        name: enemy.name.clone(),
        class_id: String::new(),
        health: enemy.hp.get(),
        max_health: enemy.hp.get(),
        mana: 0,
        max_mana: 0,
        attack: i64::from(enemy.attack.get()),
        defense: i64::from(enemy.defense.get()),
        magic_resistance: i64::from(enemy.magic_resistance.get()),
        dexterity: i64::from(enemy.dexterity.get()),
        abilities: Vec::new(),
        status_effects: Vec::new(),
        accessory: None,
        row: PartyRow::Front,
        boss: enemy.boss,
        enemy_type: Some(enemy.enemy_type),
        immunities: enemy.immunities.clone(),
        behavior: Some(enemy.behavior.clone()),
        experience_yield: enemy.experience.get(),
        enemy_size: Some(enemy.size),
        sprite_id: enemy.sprite_id().to_owned(),
        sprite_scale_percent: enemy.sprite_scale_percent.get(),
        drops: Some(enemy.drops.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_member::test_class;
    use crate::{
        new_game::{NewGameScenario, build_new_game_state},
        scenario_balance::BalanceData,
        scenario_enemy::EnemyCatalogFile,
        scenario_manifest::Manifest,
        scenario_party::PartyCatalog,
        scenario_yaml,
    };

    fn zone() -> EncounterZone {
        scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/encounter-regular-zone.yaml"
        ))
        .unwrap()
    }

    fn enemies() -> EnemyCatalog {
        let file = EnemyCatalogFile::from_yaml_stream(include_str!(
            "../../../tests/fixtures/enemy-rule-shapes.yaml"
        ))
        .unwrap();
        EnemyCatalog::try_from_definitions(file.0).unwrap()
    }

    fn game() -> crate::game_state::GameState {
        let manifest_document =
            include_str!("../../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml")
                .replacen("  id: aric", "  id: ember", 1)
                .replacen("  class: hero", "  class: vanguard", 1);
        let manifest: Manifest = scenario_yaml::from_str(&manifest_document).unwrap();
        let party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/party-catalog-shapes.yaml"
        ))
        .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/balance-complete.yaml"
        ))
        .unwrap();
        build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
                protagonist_class: &test_class(&manifest.protagonist.class),
            },
            std::time::Duration::ZERO,
        )
        .unwrap()
    }

    #[test]
    fn map_lookup_distinguishes_configured_and_no_encounter_maps() {
        let catalog =
            EncounterCatalog::try_from_zones([("zone_mossy_track".to_owned(), zone())]).unwrap();
        assert_eq!(catalog.len(), 1);
        assert!(catalog.zone_for_map("zone_mossy_track").is_some());
        assert!(catalog.zone_for_map("town_no_encounters").is_none());
    }

    #[test]
    fn weighted_selection_is_deterministic_and_never_selects_zero_weight() {
        let entries = zone().entries;
        let picks = |seed| {
            let mut rng = GameplayRng::from_seed(seed);
            (0..8)
                .map(|_| {
                    pick_weighted_formation(&entries, &mut rng)
                        .unwrap()
                        .enemy_ids
                        .clone()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(picks(42), picks(42));
        assert_ne!(picks(42), picks(43));
        assert!(picks(42).iter().all(|ids| ids.as_slice() != ["unused"]));
    }

    #[test]
    fn python_weight_boundary_fixture_matches_cumulative_selection() {
        let entries = zone().entries;
        // random.Random(42).choices over weights [60, 40, 0] lands in these buckets.
        let python_unit_rolls = [
            0.639_426_798_457_883_7,
            0.025_010_755_222_666_936,
            0.275_029_318_369_119_26,
            0.223_210_738_148_822_75,
            0.736_471_214_164_012_4,
        ];
        let picked = python_unit_rolls.map(|roll| {
            pick_weighted_formation_at(&entries, (roll * 100.0) as u64)
                .unwrap()
                .enemy_ids
                .clone()
        });
        assert_eq!(
            picked,
            [
                vec!["moss_hare".to_owned(), "reed_wisp".to_owned()],
                vec!["moss_hare".to_owned()],
                vec!["moss_hare".to_owned()],
                vec!["moss_hare".to_owned()],
                vec!["moss_hare".to_owned(), "reed_wisp".to_owned()],
            ]
        );
    }

    #[test]
    fn fixed_seed_reproduces_the_pinned_python_formation_fixture() {
        // The port's versioned SplitMix64 stream intentionally differs from Python's MT19937.
        // Seed 7 is the Rust fixture seed for the formation bucket trace captured from Python
        // random.Random(42): pair, single, single, single, pair.
        let entries = zone().entries;
        let mut rng = GameplayRng::from_seed(7);
        let picked = (0..5)
            .map(|_| {
                pick_weighted_formation(&entries, &mut rng)
                    .unwrap()
                    .enemy_ids
                    .clone()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            picked,
            [
                vec!["moss_hare".to_owned(), "reed_wisp".to_owned()],
                vec!["moss_hare".to_owned()],
                vec!["moss_hare".to_owned()],
                vec!["moss_hare".to_owned()],
                vec!["moss_hare".to_owned(), "reed_wisp".to_owned()],
            ]
        );
    }

    #[test]
    fn cadence_can_reactivate_but_never_synthesizes_contact() {
        let mut cadence = SpawnCadence::new(10.0);
        assert!(!cadence.advance(100.0, 1.0, false));
        assert!(!cadence.advance(9.99, 1.0, true));
        assert!(cadence.advance(0.01, 1.0, true));
        assert!(!cadence.advance(0.0, 1.0, true));
    }

    #[test]
    fn rogue_and_non_rogue_parties_have_expected_modifiers() {
        let game = game();
        let balance = BalanceData::default();
        assert_eq!(
            party_encounter_modifiers(game.party(), &balance.spawner),
            EncounterModifiers::default()
        );
        assert_eq!(
            party_encounter_modifiers_for([("rogue", None)], &balance.spawner),
            EncounterModifiers {
                interval_multiplier: 1.25,
                chase_range_reduction: 2,
            }
        );
        assert_eq!(
            party_encounter_modifiers_for([("rogue", Some("stealth_cloak"))], &balance.spawner,),
            EncounterModifiers {
                interval_multiplier: 1.25,
                chase_range_reduction: 5,
            }
        );
    }

    #[test]
    fn enemy_catalog_retains_stats_actions_drops_sprite_and_conditions() {
        let catalog = enemies();
        let enemy = catalog.enemy("brass_sentry").unwrap();
        assert_eq!(enemy.hp.get(), 80);
        assert_eq!(enemy.sprite_id(), "brass_sentry");
        assert_eq!(enemy.drops.mc.len(), 2);
        assert_eq!(enemy.drops.loot.len(), 1);
        assert!(matches!(
            enemy.behavior,
            crate::scenario_enemy::EnemyBehavior::Inline { .. }
        ));
        assert!(catalog.enemy("veil_wraith").unwrap().barrier.is_some());
    }

    #[test]
    fn battle_entry_copies_party_enemy_assets_audio_and_return_context() {
        let game = game();
        let zone = zone();
        let context = PreBattleReturnContext {
            map_id: "zone_mossy_track".to_owned(),
            position: Position::new(7, 9),
            facing: CardinalDirection::Left,
            world_bgm_key: Some("zone.moss".to_owned()),
            world_enemies: Vec::new(),
        };
        let entry = build_battle_entry(
            "spawn-1",
            &["moss_hare".to_owned()],
            &zone,
            &enemies(),
            &FieldMenuCatalog::default(),
            game.party(),
            game.repository(),
            game.flags(),
            false,
            context.clone(),
        )
        .unwrap();
        let party = &entry.participants[0];
        assert_eq!(party.side, BattleSide::Party);
        assert_eq!(party.enemy_size, None);
        assert_eq!(
            party.health,
            game.party().members().next().unwrap().health()
        );
        let enemy = entry
            .participants
            .iter()
            .find(|participant| participant.side == BattleSide::Enemy)
            .unwrap();
        assert_eq!(enemy.id, "moss_hare");
        assert_eq!(enemy.health, 12);
        assert_eq!(enemy.enemy_size, Some(EnemySize::Small));
        assert_eq!(entry.background_id, "moss-track-bg-1280x468");
        assert_eq!(
            entry.background_asset,
            "assets/images/battle_bg/moss-track-bg-1280x468.webp"
        );
        assert_eq!(entry.bgm_key, "battle.normal");
        assert_eq!(entry.return_context, context);
    }

    /// A zone whose boss encounter is already spoken for, to prove a scripted duel fought on the
    /// same map does not steal the boss's completion flag.
    fn zone_with_boss() -> EncounterZone {
        scenario_yaml::from_str(
            "id: zone_mossy_track\n\
             name: Mossy Track\n\
             density: 0.35\n\
             background: moss-track-bg-1280x468\n\
             boss:\n\
             \x20 id: clockwork_tyrant\n\
             \x20 on_complete:\n\
             \x20   set_flag: boss_mossy_track_defeated\n",
        )
        .unwrap()
    }

    fn scripted_context() -> PreBattleReturnContext {
        PreBattleReturnContext {
            map_id: "zone_mossy_track".to_owned(),
            position: Position::new(3, 4),
            facing: CardinalDirection::Up,
            world_bgm_key: Some("zone.moss".to_owned()),
            world_enemies: Vec::new(),
        }
    }

    /// Roadmap B1.2. The Marshal's Camp is the only scripted battle in the shipped scenario and
    /// the only thing between zone 9 and the final stronghold, so it is worth pinning against
    /// the real content rather than a fixture: the duel has to name an enemy the catalog has,
    /// that enemy has to resolve its boss move set, and the camp's zone has to be able to frame
    /// a fight it never spawns.
    #[test]
    fn the_shipped_marshal_duel_builds_a_real_one_enemy_boss_battle() {
        use crate::scenario_dialogue::DialogueDocument;

        let duel: DialogueDocument = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/dialogue/cinder_marshal_duel.yaml"
        ))
        .unwrap();
        let DialogueDocument::Entries(duel) = duel else {
            panic!("the duel is an entry document");
        };
        let named = duel
            .entries
            .iter()
            .filter_map(|entry| entry.on_complete.start_battle.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(
            named,
            ["cinder_marshal", "cinder_marshal"],
            "both refusal branches fight, and nothing else does"
        );

        let zone: EncounterZone = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/encount/zone_09_marshal_camp.yaml"
        ))
        .unwrap();
        assert_eq!(zone.background, "zone9-bg-1280x468");
        assert!(
            zone.entries.is_empty() && zone.boss.is_none(),
            "the camp spawns nothing; the zone exists only to frame the scripted fight"
        );

        let file = EnemyCatalogFile::from_yaml_stream(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_1_SS.yaml"
        ))
        .unwrap();
        let marshal = file
            .entries()
            .iter()
            .find(|enemy| enemy.id == "cinder_marshal")
            .expect("the duel's enemy must be in the shipped catalog")
            .clone();
        assert!(marshal.boss, "the duel is a boss fight");
        let EnemyBehavior::Referenced { ai_ref } = &marshal.behavior else {
            panic!("the Marshal should reference a boss move set, like every other boss");
        };
        assert_eq!(ai_ref.as_str(), "boss_move_sets/cinder_marshal.yaml");

        // The whole SS file references ten other move sets, so resolve against a catalog holding
        // just this fight rather than loading all of them to prove one link.
        let move_set: BossMoveSet = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/cinder_marshal.yaml"
        ))
        .unwrap();
        let mut catalog = EnemyCatalog::try_from_definitions([marshal]).unwrap();
        catalog
            .resolve_boss_move_sets([("boss_move_sets/cinder_marshal.yaml", &move_set)])
            .expect("the Marshal's move set must resolve");
        assert!(
            matches!(
                catalog.enemy("cinder_marshal").unwrap().behavior,
                EnemyBehavior::Inline { .. }
            ),
            "resolving should inline the move set"
        );

        let game = game();
        let entry = build_scripted_battle_entry(
            "cinder_marshal",
            &zone,
            &catalog,
            &FieldMenuCatalog::default(),
            game.party(),
            game.repository(),
            game.flags(),
            scripted_context(),
        )
        .unwrap();
        let enemies = entry
            .participants
            .iter()
            .filter(|participant| participant.side == BattleSide::Enemy)
            .collect::<Vec<_>>();
        assert_eq!(enemies.len(), 1, "a duel is one enemy");
        assert_eq!(enemies[0].id, "cinder_marshal");
        assert_eq!(entry.bgm_key, "battle.boss");
        // The camp has no boss encounter of its own to close, and the dialogue owns its flags.
        assert_eq!(entry.boss_completion_flag, None);
        assert!(entry.barrier_messages.is_empty());
    }

    #[test]
    fn a_scripted_battle_fights_exactly_the_named_enemy() {
        let game = game();
        let entry = build_scripted_battle_entry(
            "moss_hare",
            &zone(),
            &enemies(),
            &FieldMenuCatalog::default(),
            game.party(),
            game.repository(),
            game.flags(),
            scripted_context(),
        )
        .unwrap();
        let enemy_ids = entry
            .participants
            .iter()
            .filter(|participant| participant.side == BattleSide::Enemy)
            .map(|participant| participant.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(enemy_ids, ["moss_hare"]);
        assert_eq!(entry.encounter_id, "scripted:moss_hare");
        assert_eq!(entry.background_id, "moss-track-bg-1280x468");
        assert_eq!(entry.bgm_key, "battle.normal");
        assert_eq!(entry.return_context, scripted_context());
    }

    #[test]
    fn a_boss_flagged_enemy_gets_boss_music_but_never_the_zone_boss_flag() {
        let game = game();
        let zone = zone_with_boss();
        // The zone's own boss encounter still closes its flag.
        let wandering = build_battle_entry(
            "zone_mossy_track:boss",
            &["clockwork_tyrant".to_owned()],
            &zone,
            &enemies(),
            &FieldMenuCatalog::default(),
            game.party(),
            game.repository(),
            game.flags(),
            true,
            scripted_context(),
        )
        .unwrap();
        assert_eq!(
            wandering.boss_completion_flag.as_deref(),
            Some("boss_mossy_track_defeated")
        );

        let scripted = build_scripted_battle_entry(
            "clockwork_tyrant",
            &zone,
            &enemies(),
            &FieldMenuCatalog::default(),
            game.party(),
            game.repository(),
            game.flags(),
            scripted_context(),
        )
        .unwrap();
        assert_eq!(scripted.bgm_key, "battle.boss");
        assert_eq!(scripted.boss_completion_flag, None);
    }

    #[test]
    fn a_scripted_battle_rejects_an_unknown_or_barriered_enemy() {
        let game = game();
        assert_eq!(
            build_scripted_battle_entry(
                "no_such_enemy",
                &zone(),
                &enemies(),
                &FieldMenuCatalog::default(),
                game.party(),
                game.repository(),
                game.flags(),
                scripted_context(),
            ),
            Err(BuildBattleError::UnknownEnemy("no_such_enemy".to_owned()))
        );
        // A duel against an enemy this zone gates behind a key item would filter to an empty
        // formation, so it is refused by name instead of failing as `EmptyFormation`.
        let barriered: EncounterZone = scenario_yaml::from_str(
            "id: zone_mossy_track\n\
             name: Mossy Track\n\
             density: 0.35\n\
             background: moss-track-bg-1280x468\n\
             barrier_enemies:\n\
             \x20 - id: veil_wraith\n\
             \x20   requires_item: invented_reed_charm\n\
             \x20   blocked_message: The reeds turn aside your strike.\n",
        )
        .unwrap();
        assert_eq!(
            build_scripted_battle_entry(
                "veil_wraith",
                &barriered,
                &enemies(),
                &FieldMenuCatalog::default(),
                game.party(),
                game.repository(),
                game.flags(),
                scripted_context(),
            ),
            Err(BuildBattleError::BarrieredScriptedEnemy(
                "veil_wraith".to_owned()
            ))
        );
    }

    #[test]
    fn barrier_filtering_requires_inventory_and_rejects_empty_battles() {
        let game = game();
        let zone = zone();
        let context = PreBattleReturnContext {
            map_id: "zone_mossy_track".to_owned(),
            position: Position::new(0, 0),
            facing: CardinalDirection::Down,
            world_bgm_key: None,
            world_enemies: Vec::new(),
        };
        assert_eq!(
            build_battle_entry(
                "blocked",
                &["reed_wisp".to_owned()],
                &zone,
                &enemies(),
                &FieldMenuCatalog::default(),
                game.party(),
                game.repository(),
                game.flags(),
                false,
                context,
            ),
            Err(BuildBattleError::EmptyFormation)
        );
    }

    #[test]
    fn return_context_restores_map_position_and_facing_after_mutation() {
        let mut game = game();
        let context = PreBattleReturnContext {
            map_id: game.map().current().unwrap().as_str().to_owned(),
            position: Position::new(12, 7),
            facing: CardinalDirection::Right,
            world_bgm_key: Some("town.default".to_owned()),
            world_enemies: vec![WorldEnemyReturnState {
                encounter_id: "spawn:0".to_owned(),
                formation: vec!["moss_hare".to_owned()],
                origin: Position::new(4, 4),
                position: Position::new(5, 4),
                facing: CardinalDirection::Left,
                boss: false,
                chase_range: 4,
                active: false,
            }],
        };
        game.map_mut().set_position(Position::new(99, 99));
        game.map_mut().set_facing(CardinalDirection::Up);
        restore_pre_battle_context(&mut game, &context).unwrap();
        assert_eq!(game.map().position(), context.position);
        assert_eq!(game.map().facing(), context.facing);
        assert_eq!(context.world_enemies.len(), 1);
        assert!(!context.world_enemies[0].active);
    }
}
