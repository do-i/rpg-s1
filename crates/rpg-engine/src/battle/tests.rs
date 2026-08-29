use super::{
    ability::{AbilityError, ElementalAffinity, elemental_damage, resolve_ability},
    action::{BattleAction, BattleEvent},
    enemy_ai::{EnemyAction, pick_enemy_action, resolve_targets},
    item::{ItemUseError, resolve_item as resolve_battle_item},
    model::{
        BattleCombatant, BattlePhase, BattleState, CombatantKey, FleeOutcome, TargetGroup,
        TargetSelector,
    },
    resolver::resolve_action,
    rewards::{RewardError, apply_rewards, calculate_rewards},
    rules::{
        calculate_turn_order, critical_damage, critical_hit_chance, flee_chance,
        phase_after_flee_confirmation, physical_damage, physical_hit_chance, roll_flee,
        roll_succeeds,
    },
    status::{ActiveStatus, StatusEffect},
};
use crate::{
    encounter::BattleSide,
    field_menu_domain::FieldMenuCatalog,
    game_state::GameState,
    gameplay_rng::GameplayRng,
    new_game::{NewGameScenario, build_new_game_state},
    runtime_repository::RuntimeRepository,
    scenario_balance::BalanceData,
    scenario_class::{Ability, ClassDefinition},
    scenario_enemy::{BossMoveSet, EnemyBehavior, EnemyCatalogFile, EnemyDrops, EnemyType},
    scenario_item::{ConsumableItem, ItemCatalogFile, ItemDefinition},
    scenario_manifest::Manifest,
    scenario_party::{PartyCatalog, PartyRow},
    scenario_yaml,
    test_support::{assert_clean_pinned_python_source, pinned_python_source},
};

const PYTHON_BATTLE_PARITY_ORACLE: &str = r#"
from engine.battle.action_resolver import resolve_action
from engine.battle.battle_logic import attempt_flee
from engine.battle.battle_rewards import RewardCalculator
from engine.battle.battle_state import BattleState
from engine.battle.combatant import ActiveStatus, Combatant, StatusEffect
from engine.item.item_effect_handler import FieldItemDef, ItemEffectHandler
from engine.party.member_state import MemberState
from engine.party.party_state import PartyState
from engine.party.repository_state import RepositoryState
from engine.util.pseudo_random import PseudoRandom

def actor(name, *, hp=100, hp_max=100, mp=50, mp_max=50, atk=20, defense=5,
          mres=10, dex=10, enemy=False, row="front", boss=False, exp=0, drops=None):
    return Combatant(id=name.lower(), name=name, hp=hp, hp_max=hp_max,
        mp=mp, mp_max=mp_max, atk=atk, def_=defense, mres=mres, dex=dex,
        is_enemy=enemy, row=row, boss=boss, exp_yield=exp, drops=drops or {})

def physical(attacker_row="front", defender_row="front"):
    source = actor("Hero", row=attacker_row)
    target = actor("Goblin", hp=100, hp_max=100, enemy=True, row=defender_row)
    state = BattleState(party=[source], enemies=[target])
    state.pending_action = {"type": "attack", "source": source, "targets": [target]}
    resolve_action(state, 1280)
    return 100 - target.hp

print(f"PHYSICAL front={physical()} back_attack={physical('back')} "
      f"back_defend={physical(defender_row='back')} both={physical('back', 'back')}")

caster = actor("Mage", mp=30, mp_max=30, mres=15)
target = actor("Goblin", hp=50, hp_max=50, mres=3, enemy=True)
state = BattleState(party=[caster], enemies=[target])
state.pending_action = {"type": "spell", "source": caster, "targets": [target],
    "data": {"name": "Fire Bolt", "type": "spell", "spell_coeff": 1.0, "mp_cost": 4}}
resolve_action(state, 1280)
print(f"SPELL fire_bolt damage={50 - target.hp} mp={caster.mp}")

handler = ItemEffectHandler.__new__(ItemEffectHandler)
handler._defs = {"potion": FieldItemDef(id="potion", effect="restore_hp",
    target="single_alive", amount=100)}
repository = RepositoryState()
repository.add_item("potion", 5)
source = actor("Hero")
target = actor("Ally", hp=50, hp_max=200)
state = BattleState(party=[source, target], enemies=[actor("Goblin", enemy=True)])
state.pending_action = {"type": "item", "source": source, "targets": [target],
    "data": {"id": "potion"}}
resolve_action(state, 1280, effect_handler=handler, repository=repository)
print(f"ITEM potion hp={target.hp} qty={repository.get_item('potion').qty}")

target = actor("Hero", hp=7, hp_max=7)
target.add_status(ActiveStatus(effect=StatusEffect.BURN, duration_turns=1, damage_per_turn=4))
damage = target.tick_end_of_turn()
print(f"STATUS burn damage={damage} hp={target.hp} active={str(target.has_status(StatusEffect.BURN)).lower()}")

boss = actor("Boss", enemy=True, boss=True)
state = BattleState(party=[actor("Hero")], enemies=[boss])
blocked, _ = attempt_flee(state, None, PseudoRandom(1))
print(f"BOSS flee={'Success' if blocked else 'Blocked'}")

growth = {"exp_base": 100, "exp_factor": 2.0, "stat_growth": {
    "str": [1] * 10, "dex": [1] * 10, "con": [1] * 10, "int": [1] * 10}}
def member(name, hp):
    value = MemberState(member_id=name.lower(), name=name, protagonist=name == "A",
        class_name="hero", level=1, exp=0, hp=hp, hp_max=50, mp=20, mp_max=20,
        str_=10, dex=8, con=9, int_=6, equipped={})
    value.load_stat_growth(growth)
    return value
party = PartyState()
for value in [member("A", 50), member("B", 50), member("KO", 0)]:
    party.add_member(value)
enemy = actor("Slime", enemy=True, exp=7, drops={"mc": [{"size": "XS", "qty": 2}],
    "loot": [{"pool": [{"item": "rat_tail", "weight": 1}]}]})
rewards = RewardCalculator(PseudoRandom(17)).calculate([enemy], party)
awards = ",".join(str(value.exp_gained) for value in sorted(
    rewards.member_results, key=lambda value: value.exp_gained))
loot = [f"mc_{value['size'].lower()}:{value['qty']}" for value in rewards.loot.mc_drops]
loot += [f"{value['id']}:{value['qty']}" for value in rewards.loot.item_drops]
print(f"REWARD exp={rewards.total_exp} awards={awards} loot={','.join(sorted(loot))}")
"#;

fn ability(id: &str) -> Ability {
    [
        include_str!("../../../../assets/scenarios/rusted_kingdoms/data/classes/cleric.yaml"),
        include_str!("../../../../assets/scenarios/rusted_kingdoms/data/classes/hero.yaml"),
        include_str!("../../../../assets/scenarios/rusted_kingdoms/data/classes/rogue.yaml"),
        include_str!("../../../../assets/scenarios/rusted_kingdoms/data/classes/sorcerer.yaml"),
        include_str!("../../../../assets/scenarios/rusted_kingdoms/data/classes/warrior.yaml"),
    ]
    .into_iter()
    .map(|document| scenario_yaml::from_str::<ClassDefinition>(document).unwrap())
    .flat_map(|class| class.abilities)
    .find(|ability| ability.id == id)
    .unwrap_or_else(|| panic!("missing ability fixture {id}"))
}

fn enemy_behavior(document: &str) -> EnemyBehavior {
    let move_set: BossMoveSet = scenario_yaml::from_str(document).unwrap();
    EnemyBehavior::Inline {
        ai: move_set.ai,
        targeting: move_set.targeting,
    }
}

fn first_zone_enemy_behavior(id: &str) -> EnemyBehavior {
    EnemyCatalogFile::from_yaml_stream(include_str!(
        "../../../../assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_8_F.yaml"
    ))
    .unwrap()
    .0
    .into_iter()
    .find(|enemy| enemy.id == id)
    .unwrap()
    .behavior
}

fn consumable(id: &str) -> ConsumableItem {
    [
        include_str!(
            "../../../../assets/scenarios/rusted_kingdoms/data/items/consumables_recovery.yaml"
        ),
        include_str!(
            "../../../../assets/scenarios/rusted_kingdoms/data/items/consumables_status_cure.yaml"
        ),
        include_str!(
            "../../../../assets/scenarios/rusted_kingdoms/data/items/consumables_battle_throw.yaml"
        ),
    ]
    .into_iter()
    .map(|document| scenario_yaml::from_str::<ItemCatalogFile>(document).unwrap())
    .flat_map(|catalog| catalog.0)
    .find_map(|definition| match definition {
        ItemDefinition::Consumable(item) if item.id == id => Some(item),
        _ => None,
    })
    .unwrap_or_else(|| panic!("missing consumable fixture {id}"))
}

fn reward_game() -> (GameState, BalanceData) {
    let manifest: Manifest = scenario_yaml::from_str(include_str!(
        "../../../../assets/scenarios/rusted_kingdoms/manifest.yaml"
    ))
    .unwrap();
    let party: PartyCatalog = scenario_yaml::from_str(include_str!(
        "../../../../assets/scenarios/rusted_kingdoms/data/party.yaml"
    ))
    .unwrap();
    let balance: BalanceData = scenario_yaml::from_str(include_str!(
        "../../../../assets/scenarios/rusted_kingdoms/data/balance.yaml"
    ))
    .unwrap();
    let game = build_new_game_state(
        NewGameScenario {
            manifest: &manifest,
            party: &party,
            balance: &balance,
        },
        std::time::Duration::ZERO,
    )
    .unwrap();
    (game, balance)
}

fn reward_enemy(id: &str, index: usize) -> BattleCombatant {
    let definition = EnemyCatalogFile::from_yaml_stream(include_str!(
        "../../../../assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_8_F.yaml"
    ))
    .unwrap()
    .0
    .into_iter()
    .find(|enemy| enemy.id == id)
    .unwrap();
    let mut enemy = actor(
        BattleSide::Enemy,
        index,
        i64::from(definition.dexterity.get()),
        1,
    );
    enemy.id = definition.id;
    enemy.name = definition.name;
    enemy.health = 0;
    enemy.max_health = definition.hp.get();
    enemy.boss = definition.boss;
    enemy.enemy_type = Some(definition.enemy_type);
    enemy.experience_yield = definition.experience.get();
    enemy.drops = Some(definition.drops);
    enemy
}

pub(super) fn actor(side: BattleSide, index: usize, dex: i64, health: u32) -> BattleCombatant {
    BattleCombatant {
        key: CombatantKey { side, index },
        id: format!("{side:?}-{index}"),
        name: format!("{side:?} {index}"),
        class_id: String::new(),
        health,
        max_health: health.max(1),
        mana: 0,
        max_mana: 0,
        attack: 10,
        defense: 3,
        magic_resistance: 2,
        dexterity: dex,
        abilities: Vec::new(),
        status_effects: Vec::new(),
        accessory: None,
        row: PartyRow::Front,
        boss: false,
        enemy_type: None,
        immunities: Vec::new(),
        behavior: None,
        experience_yield: 0,
        drops: None,
    }
}

pub(super) fn state_with(combatants: Vec<BattleCombatant>) -> BattleState {
    BattleState {
        phase: BattlePhase::Command,
        turn_order: calculate_turn_order(&combatants),
        combatants,
        active_turn: 0,
        turn_count: 1,
        command_index: 0,
        ability_index: 0,
        pending_ability: None,
        item_index: 0,
        item_choices: Vec::new(),
        pending_item: None,
        target: None,
        message: String::new(),
        transcript: Vec::new(),
        feedback_events: Vec::new(),
        used_enemy_moves: std::collections::HashSet::new(),
        rewards: None,
        flee_outcome: None,
    }
}

/// The `GlobalZIndex` of every descendant of `root` that would be drawn *behind* the battle screen.
///
/// `bevy_ui::stack` walks a root's children filtered by `Without<GlobalZIndex>` and re-sorts the
/// ones that do carry it against the roots themselves. So a descendant global index at or below
/// [`super::ui::BATTLE_ROOT_Z`] does not sit lower *within* the battle screen — it sits under the
/// root's own opaque floor, and the player never sees it. Nothing about layout, colour or
/// visibility changes, which is why a whole feature could be hidden this way while every test
/// asserting on `Node` and `BackgroundColor` stayed green.
pub(super) fn hidden_behind_the_battle_root(
    world: &bevy::prelude::World,
    root: bevy::prelude::Entity,
) -> Vec<i32> {
    use bevy::prelude::{Children, GlobalZIndex};

    let mut sunk = Vec::new();
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if entity != root
            && let Some(zindex) = world.get::<GlobalZIndex>(entity)
            && zindex.0 <= super::ui::BATTLE_ROOT_Z
        {
            sunk.push(zindex.0);
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    sunk
}

#[test]
fn phase_graph_names_and_bounds_every_minimum_loop_transition() {
    assert!(BattlePhase::Start.allows(BattlePhase::Command));
    assert!(BattlePhase::Command.allows(BattlePhase::Ability));
    assert!(BattlePhase::Ability.allows(BattlePhase::Target));
    assert!(BattlePhase::Target.allows(BattlePhase::Ability));
    assert!(BattlePhase::Command.allows(BattlePhase::Item));
    assert!(BattlePhase::Item.allows(BattlePhase::Target));
    assert!(BattlePhase::Target.allows(BattlePhase::Item));
    assert!(BattlePhase::Command.allows(BattlePhase::Target));
    assert!(BattlePhase::Target.allows(BattlePhase::Resolve));
    assert!(BattlePhase::Resolve.allows(BattlePhase::Advance));
    assert!(BattlePhase::Advance.allows(BattlePhase::Command));
    assert!(BattlePhase::Resolve.allows(BattlePhase::Victory));
    assert!(BattlePhase::Victory.allows(BattlePhase::Rewards));
    assert!(BattlePhase::Resolve.allows(BattlePhase::Defeat));
    assert!(BattlePhase::Command.allows(BattlePhase::Flee));
    assert!(BattlePhase::Flee.allows(BattlePhase::Advance));
    assert!(!BattlePhase::Victory.allows(BattlePhase::Command));
    assert!(!BattlePhase::Rewards.allows(BattlePhase::Command));
    assert!(!BattlePhase::Defeat.allows(BattlePhase::Advance));
}

#[test]
fn elemental_affinity_scales_weak_neutral_and_resistant_damage() {
    assert_eq!(elemental_damage(11, ElementalAffinity::Weak), 16);
    assert_eq!(elemental_damage(11, ElementalAffinity::Neutral), 11);
    assert_eq!(elemental_damage(11, ElementalAffinity::Resistant), 5);
    assert_eq!(elemental_damage(1, ElementalAffinity::Resistant), 1);
}

#[test]
fn offensive_spell_charges_once_and_resolves_single_and_all_target_shapes() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.magic_resistance = 15;
    caster.mana = 30;
    caster.max_mana = 30;
    caster.abilities = vec![ability("fire_bolt"), ability("flame_wall")];
    let mut first = actor(BattleSide::Enemy, 0, 1, 50);
    first.magic_resistance = 3;
    let second = actor(BattleSide::Enemy, 1, 1, 50);
    let mut state = state_with(vec![caster, first, second]);

    let events = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::enemy(0)],
        &mut GameplayRng::from_seed(2),
    )
    .unwrap();
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, 26);
    assert_eq!(state.actor(CombatantKey::enemy(0)).unwrap().health, 38);
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::MagicDamage { amount: 12, .. }]
    ));

    state.phase = BattlePhase::Ability;
    let events = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        1,
        &[CombatantKey::enemy(0), CombatantKey::enemy(1)],
        &mut GameplayRng::from_seed(3),
    )
    .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, BattleEvent::MagicDamage { .. }))
            .count(),
        2
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, 6);
}

#[test]
fn holy_spell_uses_enemy_taxonomy_affinity() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.magic_resistance = 20;
    caster.mana = 20;
    caster.max_mana = 20;
    let mut spell = ability("earth_spike");
    if let crate::scenario_class::AbilityKind::Spell(value) = &mut spell.kind {
        value.element = crate::scenario_class::AbilityElement::Holy;
    }
    caster.abilities.push(spell);
    let mut undead = actor(BattleSide::Enemy, 0, 1, 50);
    undead.magic_resistance = 4;
    undead.enemy_type = Some(EnemyType::Undead);
    let mut state = state_with(vec![caster, undead]);

    let events = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::enemy(0)],
        &mut GameplayRng::from_seed(4),
    )
    .unwrap();
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::MagicDamage { amount: 24, .. }]
    ));
}

#[test]
fn invalid_casts_do_not_spend_mana_or_mutate_targets() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.mana = 3;
    caster.max_mana = 30;
    caster.abilities.push(ability("fire_bolt"));
    let enemy = actor(BattleSide::Enemy, 0, 1, 50);
    let mut state = state_with(vec![caster, enemy]);

    assert_eq!(
        resolve_ability(
            &mut state,
            CombatantKey::party(0),
            0,
            &[CombatantKey::enemy(0)],
            &mut GameplayRng::from_seed(1),
        ),
        Err(AbilityError::InsufficientMana)
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, 3);
    assert_eq!(state.actor(CombatantKey::enemy(0)).unwrap().health, 50);

    state.actor_mut(CombatantKey::party(0)).unwrap().mana = 30;
    assert_eq!(
        resolve_ability(
            &mut state,
            CombatantKey::party(0),
            0,
            &[CombatantKey::party(0)],
            &mut GameplayRng::from_seed(1),
        ),
        Err(AbilityError::InvalidTarget)
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, 30);
}

#[test]
fn healing_caps_living_hp_and_revive_only_accepts_ko_allies() {
    let mut caster = actor(BattleSide::Party, 0, 10, 80);
    caster.magic_resistance = 20;
    caster.mana = 40;
    caster.max_mana = 40;
    caster.abilities = vec![ability("heal"), ability("revive")];
    let mut living = actor(BattleSide::Party, 1, 5, 100);
    living.health = 90;
    let mut fallen = actor(BattleSide::Party, 2, 4, 100);
    fallen.health = 0;
    let mut state = state_with(vec![caster, living, fallen]);

    let heal = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::party(1)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert!(matches!(
        heal.as_slice(),
        [BattleEvent::Heal {
            amount: 10,
            revived: false,
            ..
        }]
    ));
    assert_eq!(state.actor(CombatantKey::party(1)).unwrap().health, 100);

    state.phase = BattlePhase::Ability;
    let revive = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        1,
        &[CombatantKey::party(2)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert!(matches!(
        revive.as_slice(),
        [BattleEvent::Heal {
            amount: 50,
            revived: true,
            ..
        }]
    ));
    let mana = state.actor(CombatantKey::party(0)).unwrap().mana;
    assert_eq!(
        resolve_ability(
            &mut state,
            CombatantKey::party(0),
            1,
            &[CombatantKey::party(1)],
            &mut GameplayRng::from_seed(1),
        ),
        Err(AbilityError::InvalidTarget)
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, mana);
}

#[test]
fn buffs_debuffs_refresh_and_expire_at_end_of_affected_turn() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.mana = 30;
    caster.max_mana = 30;
    caster.abilities = vec![ability("rally"), ability("war_cry")];
    let ally = actor(BattleSide::Party, 1, 5, 40);
    let enemy = actor(BattleSide::Enemy, 0, 1, 40);
    let mut state = state_with(vec![caster, ally, enemy]);
    state.actor_mut(CombatantKey::party(1)).unwrap().defense = 10;

    resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::party(0), CombatantKey::party(1)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert_eq!(
        state
            .actor(CombatantKey::party(1))
            .unwrap()
            .effective_defense(),
        12
    );
    let defense = state.actor(CombatantKey::party(1)).unwrap().status_effects[0];
    assert_eq!(defense.remaining_turns, Some(3));

    state.phase = BattlePhase::Ability;
    resolve_ability(
        &mut state,
        CombatantKey::party(0),
        1,
        &[CombatantKey::enemy(0)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert_eq!(
        state
            .actor(CombatantKey::enemy(0))
            .unwrap()
            .effective_attack(),
        8
    );
    assert_eq!(
        state
            .actor_mut(CombatantKey::enemy(0))
            .unwrap()
            .tick_statuses()
            .expired,
        0
    );
    assert_eq!(
        state
            .actor_mut(CombatantKey::enemy(0))
            .unwrap()
            .tick_statuses()
            .expired,
        1
    );
    assert_eq!(
        state
            .actor(CombatantKey::enemy(0))
            .unwrap()
            .effective_attack(),
        10
    );
}

#[test]
fn construct_taxonomy_rejects_harmful_debuff_statuses() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.mana = 10;
    caster.max_mana = 10;
    caster.abilities.push(ability("war_cry"));
    let mut construct = actor(BattleSide::Enemy, 0, 1, 40);
    construct.enemy_type = Some(EnemyType::Construct);
    let mut state = state_with(vec![caster, construct]);

    let events = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::enemy(0)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert!(events.is_empty());
    assert!(
        state
            .actor(CombatantKey::enemy(0))
            .unwrap()
            .status_effects
            .is_empty()
    );
}

#[test]
fn defensive_buffs_reduce_or_redirect_physical_damage() {
    let mut warrior = actor(BattleSide::Party, 0, 20, 100);
    warrior.abilities = vec![ability("cover"), ability("fortress_stance")];
    let ally = actor(BattleSide::Party, 1, 10, 100);
    let mut enemy = actor(BattleSide::Enemy, 0, 100, 100);
    enemy.attack = 23;
    let mut state = state_with(vec![warrior, ally, enemy]);

    resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::party(1)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    state.resolve_physical(
        CombatantKey::enemy(0),
        CombatantKey::party(1),
        &mut GameplayRng::from_seed(9),
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().health, 80);
    assert_eq!(state.actor(CombatantKey::party(1)).unwrap().health, 100);

    state.phase = BattlePhase::Ability;
    resolve_ability(
        &mut state,
        CombatantKey::party(0),
        1,
        &[CombatantKey::party(0)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    state.resolve_physical(
        CombatantKey::enemy(0),
        CombatantKey::party(0),
        &mut GameplayRng::from_seed(9),
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().health, 70);
    assert_eq!(
        state
            .actor(CombatantKey::party(0))
            .unwrap()
            .skip_turn_reason(),
        Some(StatusEffect::DamageReduction)
    );
}

#[test]
fn sleep_wakes_on_damage_stun_skips_and_silence_hides_abilities() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.abilities.push(ability("heal"));
    caster.add_status(ActiveStatus::timed(StatusEffect::Silence, 2));
    let mut state = state_with(vec![caster, actor(BattleSide::Enemy, 0, 1, 40)]);
    assert!(!state.command_available(super::model::BattleCommand::Spell));

    let caster = state.actor_mut(CombatantKey::party(0)).unwrap();
    caster.remove_status(StatusEffect::Silence);
    caster.add_status(ActiveStatus::timed(StatusEffect::Sleep, 2));
    assert_eq!(caster.skip_turn_reason(), Some(StatusEffect::Sleep));
    caster.apply_damage(1);
    assert!(!caster.has_status(StatusEffect::Sleep));
    caster.add_status(ActiveStatus::timed(StatusEffect::Stun, 1));
    assert_eq!(caster.skip_turn_reason(), Some(StatusEffect::Stun));
    assert_eq!(caster.tick_statuses().expired, 1);
    assert!(caster.skip_turn_reason().is_none());
}

#[test]
fn recovery_items_cap_pools_consume_once_and_reject_invalid_targets_atomically() {
    let mut user = actor(BattleSide::Party, 0, 10, 100);
    user.mana = 10;
    user.max_mana = 40;
    let mut ally = actor(BattleSide::Party, 1, 5, 200);
    ally.health = 150;
    let enemy = actor(BattleSide::Enemy, 0, 1, 100);
    let mut state = state_with(vec![user, ally, enemy]);
    let mut repository = RuntimeRepository::default();
    let _ = repository.add_item("potion", 2).unwrap();
    let _ = repository.add_item("ether", 1).unwrap();

    let events = resolve_battle_item(
        &mut state,
        CombatantKey::party(0),
        &consumable("potion"),
        CombatantKey::party(1),
        &mut repository,
    )
    .unwrap();
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::Heal { amount: 50, .. }]
    ));
    assert_eq!(repository.item_count("potion"), 1);

    assert_eq!(
        resolve_battle_item(
            &mut state,
            CombatantKey::party(0),
            &consumable("potion"),
            CombatantKey::enemy(0),
            &mut repository,
        ),
        Err(ItemUseError::InvalidTarget)
    );
    assert_eq!(repository.item_count("potion"), 1);

    state.phase = BattlePhase::Item;
    let events = resolve_battle_item(
        &mut state,
        CombatantKey::party(0),
        &consumable("ether"),
        CombatantKey::party(0),
        &mut repository,
    )
    .unwrap();
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::ManaRestored { amount: 30, .. }]
    ));
    assert_eq!(repository.item_count("ether"), 0);
}

#[test]
fn revive_and_cure_items_apply_authored_effects_and_consume_once() {
    let user = actor(BattleSide::Party, 0, 10, 100);
    let mut fallen = actor(BattleSide::Party, 1, 5, 200);
    fallen.health = 0;
    fallen.add_status(ActiveStatus::damage_over_time(
        StatusEffect::Poison,
        None,
        4,
    ));
    let mut state = state_with(vec![user, fallen, actor(BattleSide::Enemy, 0, 1, 100)]);
    let mut repository = RuntimeRepository::default();
    let _ = repository.add_item("life_crystal", 1).unwrap();
    let _ = repository.add_item("antidote", 1).unwrap();

    let events = resolve_battle_item(
        &mut state,
        CombatantKey::party(0),
        &consumable("life_crystal"),
        CombatantKey::party(1),
        &mut repository,
    )
    .unwrap();
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::Heal {
            amount: 200,
            revived: true,
            ..
        }]
    ));
    assert_eq!(repository.item_count("life_crystal"), 0);

    state.phase = BattlePhase::Item;
    let events = resolve_battle_item(
        &mut state,
        CombatantKey::party(0),
        &consumable("antidote"),
        CombatantKey::party(1),
        &mut repository,
    )
    .unwrap();
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::StatusCured {
            effect: Some(StatusEffect::Poison),
            ..
        }]
    ));
    assert!(
        !state
            .actor(CombatantKey::party(1))
            .unwrap()
            .has_status(StatusEffect::Poison)
    );
    assert_eq!(repository.item_count("antidote"), 0);
}

#[test]
fn elemental_throw_items_damage_one_enemy_and_apply_authored_trait_bonus() {
    let user = actor(BattleSide::Party, 0, 10, 100);
    let mut living = actor(BattleSide::Enemy, 0, 1, 400);
    living.enemy_type = Some(EnemyType::Humanoid);
    let mut undead = actor(BattleSide::Enemy, 1, 1, 400);
    undead.enemy_type = Some(EnemyType::Undead);
    let mut state = state_with(vec![user, living, undead]);
    let mut repository = RuntimeRepository::default();
    let _ = repository.add_item("holy_water", 2).unwrap();

    let ordinary = resolve_battle_item(
        &mut state,
        CombatantKey::party(0),
        &consumable("holy_water"),
        CombatantKey::enemy(0),
        &mut repository,
    )
    .unwrap();
    assert!(matches!(
        ordinary.as_slice(),
        [BattleEvent::ItemDamage { amount: 200, .. }]
    ));

    state.phase = BattlePhase::Item;
    let vulnerable = resolve_battle_item(
        &mut state,
        CombatantKey::party(0),
        &consumable("holy_water"),
        CombatantKey::enemy(1),
        &mut repository,
    )
    .unwrap();
    assert!(matches!(
        vulnerable.as_slice(),
        [BattleEvent::ItemDamage { amount: 300, .. }]
    ));
    assert_eq!(repository.item_count("holy_water"), 0);
}

#[test]
fn poison_reapplication_refreshes_damage_ticks_can_ko_and_cure_stops_future_ticks() {
    let mut target = actor(BattleSide::Party, 0, 10, 7);
    assert!(target.add_status(ActiveStatus::damage_over_time(
        StatusEffect::Poison,
        None,
        2,
    )));
    assert!(target.add_status(ActiveStatus::damage_over_time(
        StatusEffect::Poison,
        None,
        4,
    )));
    assert_eq!(target.status_effects.len(), 1);
    assert_eq!(target.tick_statuses().damage, 4);
    assert_eq!(target.health, 3);
    assert!(target.remove_status(StatusEffect::Poison));
    assert_eq!(target.tick_statuses().damage, 0);
    target.add_status(ActiveStatus::damage_over_time(
        StatusEffect::Poison,
        None,
        4,
    ));
    assert_eq!(target.tick_statuses().damage, 3);
    assert!(!target.is_alive());
}

#[test]
fn first_zone_venom_ability_applies_source_scaled_persistent_poison() {
    let party = actor(BattleSide::Party, 0, 5, 100);
    let mut boss = actor(BattleSide::Enemy, 0, 100, 100);
    boss.attack = 20;
    boss.behavior = Some(enemy_behavior(
        r#"
ai:
  pattern: random
  moves: [{ action: ability, id: venom_bite, weight: 1 }]
targeting: { default: random_alive }
"#,
    ));
    let mut state = state_with(vec![party, boss]);
    state.active_turn = state
        .turn_order
        .iter()
        .position(|key| *key == CombatantKey::enemy(0))
        .unwrap();

    state.resolve_enemy_action(&mut GameplayRng::from_seed(1));

    let target = state.actor(CombatantKey::party(0)).unwrap();
    assert!(target.has_status(StatusEffect::Poison));
    assert_eq!(
        target
            .status_effects
            .iter()
            .find(|status| status.effect == StatusEffect::Poison)
            .unwrap()
            .potency,
        super::status::StatusPotency::DamagePerTurn(2)
    );
}

#[test]
fn weighted_enemy_actions_repeat_and_stay_within_authored_moves() {
    let mut enemy = actor(BattleSide::Enemy, 0, 10, 40);
    enemy.behavior = Some(first_zone_enemy_behavior("goblin"));
    let state = state_with(vec![actor(BattleSide::Party, 0, 5, 40), enemy.clone()]);
    let sequence = |seed| {
        let mut rng = GameplayRng::from_seed(seed);
        (0..32)
            .map(|_| pick_enemy_action(&enemy, &state, &mut rng))
            .collect::<Vec<_>>()
    };
    let actions = sequence(91);
    assert_eq!(actions, sequence(91));
    assert!(actions.contains(&EnemyAction::Attack));
    assert!(actions.contains(&EnemyAction::Ability {
        id: "scratch".to_owned(),
        once: false,
    }));
}

#[test]
fn conditional_enemy_actions_filter_hp_turn_and_once_per_battle_moves() {
    let mut enemy = actor(BattleSide::Enemy, 0, 10, 100);
    enemy.boss = true;
    enemy.behavior = Some(enemy_behavior(
        r#"
ai:
  pattern: conditional
  moves:
    - action: ability
      id: desperation
      weight: 1
      condition: { hp_pct_below: 0.50 }
    - action: ability
      id: scripted_once
      weight: 1
      condition: { turn_mod: { every: 2 } }
      once: true
targeting: { default: random_alive }
"#,
    ));
    let mut state = state_with(vec![actor(BattleSide::Party, 0, 5, 40), enemy.clone()]);
    state.turn_count = 1;
    assert_eq!(
        pick_enemy_action(&enemy, &state, &mut GameplayRng::from_seed(1)),
        EnemyAction::Attack
    );

    state.turn_count = 2;
    assert_eq!(
        pick_enemy_action(&enemy, &state, &mut GameplayRng::from_seed(1)),
        EnemyAction::Ability {
            id: "scripted_once".to_owned(),
            once: true,
        }
    );
    state
        .used_enemy_moves
        .insert((CombatantKey::enemy(0), "scripted_once".to_owned()));
    assert_eq!(
        pick_enemy_action(&enemy, &state, &mut GameplayRng::from_seed(1)),
        EnemyAction::Attack
    );

    enemy.health = 30;
    state.turn_count = 3;
    assert_eq!(
        pick_enemy_action(&enemy, &state, &mut GameplayRng::from_seed(1)),
        EnemyAction::Ability {
            id: "desperation".to_owned(),
            once: false,
        }
    );
}

#[test]
fn enemy_targeting_honors_taunt_single_modes_and_all_party_override() {
    let mut low_hp = actor(BattleSide::Party, 0, 4, 100);
    low_hp.health = 10;
    let mut taunter = actor(BattleSide::Party, 1, 20, 100);
    taunter.health = 80;
    taunter.add_status(ActiveStatus::timed(StatusEffect::Taunt, 2));
    let mut enemy = actor(BattleSide::Enemy, 0, 10, 100);
    enemy.behavior = Some(enemy_behavior(
        r#"
ai:
  pattern: random
  moves: [{ action: attack, weight: 1 }]
targeting:
  default: lowest_hp
  overrides:
    - { ability: wave, target: all_party }
    - { ability: shot, target: highest_dex }
"#,
    ));
    let state = state_with(vec![low_hp, taunter, enemy.clone()]);

    assert_eq!(
        resolve_targets(&enemy, &state, "", &mut GameplayRng::from_seed(1)),
        [CombatantKey::party(1)]
    );
    assert_eq!(
        resolve_targets(&enemy, &state, "wave", &mut GameplayRng::from_seed(1)),
        [CombatantKey::party(0), CombatantKey::party(1)]
    );
    assert_eq!(
        resolve_targets(&enemy, &state, "shot", &mut GameplayRng::from_seed(1)),
        [CombatantKey::party(1)]
    );
}

#[test]
fn enemy_aoe_ability_damages_every_living_party_member_at_full_row_strength() {
    let mut front = actor(BattleSide::Party, 0, 5, 100);
    front.defense = 4;
    let mut back = actor(BattleSide::Party, 1, 4, 100);
    back.defense = 4;
    back.row = PartyRow::Back;
    let mut enemy = actor(BattleSide::Enemy, 0, 100, 100);
    enemy.attack = 14;
    enemy.behavior = Some(enemy_behavior(
        r#"
ai:
  pattern: random
  moves: [{ action: ability, id: wave, weight: 1 }]
targeting:
  default: random_alive
  overrides: [{ ability: wave, target: all_party }]
"#,
    ));
    let mut state = state_with(vec![front, back, enemy]);
    state.active_turn = state
        .turn_order
        .iter()
        .position(|key| *key == CombatantKey::enemy(0))
        .unwrap();

    state.resolve_enemy_action(&mut GameplayRng::from_seed(8));

    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().health, 90);
    assert_eq!(state.actor(CombatantKey::party(1)).unwrap().health, 90);
    assert_eq!(
        state
            .feedback_events
            .iter()
            .filter(|event| matches!(event, BattleEvent::EnemyAbilityDamage { .. }))
            .count(),
        2
    );
    assert_eq!(state.phase, BattlePhase::Resolve);
}

#[test]
fn boss_accessory_override_blocks_or_reduces_target_to_one_hp() {
    let behavior = enemy_behavior(
        r#"
ai:
  pattern: random
  moves: [{ action: ability, id: death_gaze, weight: 1 }]
targeting:
  default: random_alive
  overrides:
    - ability: death_gaze
      target: random_alive
      blocked_by_accessory: holy_talisman
      on_blocked: no_effect
      on_hit: hp_to_1
"#,
    );
    let battle = |accessory: Option<&str>| {
        let mut party = actor(BattleSide::Party, 0, 5, 100);
        party.accessory = accessory.map(str::to_owned);
        let mut boss = actor(BattleSide::Enemy, 0, 100, 100);
        boss.boss = true;
        boss.behavior = Some(behavior.clone());
        let mut state = state_with(vec![party, boss]);
        state.active_turn = state
            .turn_order
            .iter()
            .position(|key| *key == CombatantKey::enemy(0))
            .unwrap();
        state.resolve_enemy_action(&mut GameplayRng::from_seed(1));
        state
    };

    let blocked = battle(Some("holy_talisman"));
    assert_eq!(blocked.actor(CombatantKey::party(0)).unwrap().health, 100);
    assert!(matches!(
        blocked.feedback_events.as_slice(),
        [BattleEvent::EnemyAbilityBlocked { .. }]
    ));

    let hit = battle(None);
    assert_eq!(hit.actor(CombatantKey::party(0)).unwrap().health, 1);
    assert!(matches!(
        hit.feedback_events.as_slice(),
        [BattleEvent::EnemyAbilityDamage { amount: 99, .. }]
    ));
}

#[test]
fn battle_round_counter_increments_only_when_turn_order_wraps() {
    let party = actor(BattleSide::Party, 0, 10, 100);
    let enemy = actor(BattleSide::Enemy, 0, 5, 100);
    let mut state = state_with(vec![party, enemy]);
    let mut rng = GameplayRng::from_seed(1);
    assert_eq!(state.turn_count, 1);
    state.advance(&mut rng);
    assert_eq!(state.turn_count, 1);
    state.assess_result();
    state.advance(&mut rng);
    assert_eq!(state.turn_count, 2);
}

#[test]
fn rewards_split_all_exp_only_across_living_members_with_seeded_remainder() {
    let mut first = actor(BattleSide::Party, 0, 10, 100);
    first.id = "first".to_owned();
    let mut second = actor(BattleSide::Party, 1, 9, 100);
    second.id = "second".to_owned();
    let mut ko = actor(BattleSide::Party, 2, 8, 100);
    ko.id = "ko".to_owned();
    ko.health = 0;
    let mut enemy_a = actor(BattleSide::Enemy, 0, 2, 1);
    enemy_a.experience_yield = 4;
    enemy_a.health = 0;
    let mut enemy_b = actor(BattleSide::Enemy, 1, 1, 1);
    enemy_b.experience_yield = 3;
    enemy_b.health = 0;
    let state = state_with(vec![first, second, ko, enemy_a, enemy_b]);

    let rewards = calculate_rewards(&state, &mut GameplayRng::from_seed(17));
    assert_eq!(rewards.total_experience, 7);
    assert_eq!(
        rewards
            .members
            .iter()
            .map(|member| member.experience_gained)
            .sum::<u32>(),
        7
    );
    assert_eq!(
        rewards
            .members
            .iter()
            .find(|member| member.member_id == "ko")
            .unwrap()
            .experience_gained,
        0
    );
    assert_eq!(
        rewards,
        calculate_rewards(&state, &mut GameplayRng::from_seed(17))
    );
}

#[test]
fn multi_enemy_loot_aggregates_guaranteed_cores_and_seeded_pool_rolls() {
    let state = state_with(vec![
        actor(BattleSide::Party, 0, 10, 100),
        reward_enemy("goblin", 0),
        reward_enemy("grik_the_grin", 1),
    ]);
    let rewards = calculate_rewards(&state, &mut GameplayRng::from_seed(31));
    let quantity = |id: &str| {
        rewards
            .loot
            .iter()
            .find(|loot| loot.item_id == id)
            .map_or(0, |loot| loot.quantity)
    };
    assert_eq!(quantity("mc_xs"), 5);
    assert_eq!(quantity("mc_s"), 1);
    assert_eq!(
        rewards
            .loot
            .iter()
            .filter(|loot| !loot.magic_core)
            .map(|loot| loot.quantity)
            .sum::<u32>(),
        2
    );
    assert_eq!(
        rewards,
        calculate_rewards(&state, &mut GameplayRng::from_seed(31))
    );
}

#[test]
fn reward_application_is_atomic_one_time_and_sets_only_a_defeated_boss_flag() {
    let (mut game, balance) = reward_game();
    let catalog = FieldMenuCatalog::production_class_fixture();
    let mut aric = actor(BattleSide::Party, 0, 10, 22);
    aric.id = "aric".to_owned();
    aric.name = "Aric".to_owned();
    aric.class_id = "hero".to_owned();
    aric.health = 12;
    aric.mana = 2;
    aric.max_mana = 12;
    let mut boss = reward_enemy("grik_the_grin", 0);
    boss.experience_yield = 400;
    let mut state = state_with(vec![aric, boss]);

    let rewards = apply_rewards(
        &mut state,
        &mut game,
        &catalog,
        &balance,
        Some("boss_zone01_defeated"),
    )
    .unwrap();
    let member = game.party().member("aric").unwrap();
    assert_eq!((member.level(), member.experience()), (2, 400));
    assert_eq!((member.health(), member.mana()), (59, 24));
    assert_eq!(
        (
            state.actor(CombatantKey::party(0)).unwrap().health,
            state.actor(CombatantKey::party(0)).unwrap().max_health,
            state.actor(CombatantKey::party(0)).unwrap().mana,
            state.actor(CombatantKey::party(0)).unwrap().max_mana,
        ),
        (59, 59, 24, 24)
    );
    assert_eq!(rewards.members[0].learned_abilities, ["Power Strike"]);
    assert!(game.flags().is_set("boss_zone01_defeated"));
    assert_eq!(game.repository().item_count("mc_xs"), 2);
    assert_eq!(game.repository().item_count("mc_s"), 1);
    assert!(game.repository().is_loot("mc_xs"));
    assert_eq!(
        game.repository().item_tags("mc_xs").collect::<Vec<_>>(),
        ["magic_core"]
    );
    assert_eq!(rewards.gp_gained, 0);
    let summary = rewards.summary_lines();
    assert_eq!(summary[0], "EXP 400  GP 0");
    assert_eq!(summary[1], "Aric +400 EXP");
    assert!(summary[2].contains("Magic Core (XS) x2"));
    assert!(summary[2].contains("Magic Core (S) x1"));
    assert!(summary[3].contains("Aric Lv 1>2"));
    assert!(summary[3].contains("Aric learned Power Strike"));
    assert!(summary[3].contains("Boss cleared (boss_zone01_defeated)"));
    let detail = rewards.detail_message();
    assert!(detail.contains("Aric Lv 1>2\nHP +37=59  MP +12=24"));
    assert!(detail.contains("STR +2=30  DEX +2=19  CON +3=31  INT +1=6"));

    let after = game.clone();
    assert!(matches!(
        apply_rewards(
            &mut state,
            &mut game,
            &catalog,
            &balance,
            Some("boss_zone01_defeated"),
        ),
        Err(RewardError::AlreadyApplied)
    ));
    assert_eq!(game, after);
}

#[test]
fn configured_boss_flag_is_ignored_for_a_regular_enemy_victory() {
    let (mut game, balance) = reward_game();
    let catalog = FieldMenuCatalog::production_class_fixture();
    let mut aric = actor(BattleSide::Party, 0, 10, 22);
    aric.id = "aric".to_owned();
    aric.class_id = "hero".to_owned();
    let enemy = reward_enemy("goblin", 0);
    let mut state = state_with(vec![aric, enemy]);

    let rewards = apply_rewards(
        &mut state,
        &mut game,
        &catalog,
        &balance,
        Some("should_not_set"),
    )
    .unwrap();

    assert_eq!(rewards.boss_flag, None);
    assert!(!game.flags().is_set("should_not_set"));
}

fn full_battle_parity_transcript() -> Vec<String> {
    let mut attacker = actor(BattleSide::Party, 0, 10, 100);
    attacker.attack = 20;
    let mut defender = actor(BattleSide::Enemy, 0, 5, 100);
    defender.defense = 5;
    let front = physical_damage(&attacker, &defender);
    attacker.row = PartyRow::Back;
    let back_attack = physical_damage(&attacker, &defender);
    attacker.row = PartyRow::Front;
    defender.key = CombatantKey::party(1);
    defender.row = PartyRow::Back;
    let back_defend = physical_damage(&attacker, &defender);
    attacker.row = PartyRow::Back;
    let both = physical_damage(&attacker, &defender);

    let mut caster = actor(BattleSide::Party, 0, 10, 100);
    caster.id = "mage".to_owned();
    caster.magic_resistance = 15;
    caster.mana = 30;
    caster.max_mana = 30;
    caster.abilities = vec![ability("fire_bolt")];
    let mut spell_target = actor(BattleSide::Enemy, 0, 5, 50);
    spell_target.magic_resistance = 3;
    let mut spell_state = state_with(vec![caster, spell_target]);
    resolve_ability(
        &mut spell_state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::enemy(0)],
        &mut GameplayRng::from_seed(2),
    )
    .unwrap();
    let spell_damage = 50 - spell_state.actor(CombatantKey::enemy(0)).unwrap().health;
    let spell_mana = spell_state.actor(CombatantKey::party(0)).unwrap().mana;

    let (_, balance) = reward_game();
    let mut repository = RuntimeRepository::from_balance(&balance.economy);
    let _ = repository.add_item("potion", 5).unwrap();
    let source = actor(BattleSide::Party, 0, 10, 100);
    let mut item_target = actor(BattleSide::Party, 1, 9, 200);
    item_target.health = 50;
    let mut item_state = state_with(vec![source, item_target]);
    resolve_battle_item(
        &mut item_state,
        CombatantKey::party(0),
        &consumable("potion"),
        CombatantKey::party(1),
        &mut repository,
    )
    .unwrap();

    let mut status_target = actor(BattleSide::Party, 0, 10, 7);
    status_target.add_status(ActiveStatus::damage_over_time(
        StatusEffect::Burn,
        Some(1),
        4,
    ));
    let status_tick = status_target.tick_statuses();

    let boss_flee = roll_flee(true, 1.0, &mut GameplayRng::from_seed(1));

    let mut first = actor(BattleSide::Party, 0, 10, 50);
    first.id = "a".to_owned();
    let mut second = actor(BattleSide::Party, 1, 9, 50);
    second.id = "b".to_owned();
    let mut ko = actor(BattleSide::Party, 2, 8, 50);
    ko.id = "ko".to_owned();
    ko.health = 0;
    let mut enemy = actor(BattleSide::Enemy, 0, 1, 1);
    enemy.health = 0;
    enemy.experience_yield = 7;
    enemy.drops = Some(
        scenario_yaml::from_str::<EnemyDrops>(
            "mc: [{size: XS, qty: 2}]\nloot: [{pool: [{item: rat_tail, weight: 1}]}]\n",
        )
        .unwrap(),
    );
    let reward_state = state_with(vec![first, second, ko, enemy]);
    let rewards = calculate_rewards(&reward_state, &mut GameplayRng::from_seed(17));
    let mut awards = rewards
        .members
        .iter()
        .map(|member| member.experience_gained)
        .collect::<Vec<_>>();
    awards.sort_unstable();
    let loot = rewards
        .loot
        .iter()
        .map(|drop| format!("{}:{}", drop.item_id, drop.quantity))
        .collect::<Vec<_>>()
        .join(",");

    vec![
        format!(
            "PHYSICAL front={front} back_attack={back_attack} back_defend={back_defend} both={both}"
        ),
        format!("SPELL fire_bolt damage={spell_damage} mp={spell_mana}"),
        format!(
            "ITEM potion hp={} qty={}",
            item_state.actor(CombatantKey::party(1)).unwrap().health,
            repository.item_count("potion")
        ),
        format!(
            "STATUS burn damage={} hp={} active={}",
            status_tick.damage,
            status_target.health,
            status_target.has_status(StatusEffect::Burn)
        ),
        format!("BOSS flee={boss_flee:?}"),
        format!(
            "REWARD exp={} awards={} loot={loot}",
            rewards.total_experience,
            awards
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
    ]
}

#[test]
fn full_battle_parity_transcript_is_pinned_without_python_installed() {
    assert_eq!(
        full_battle_parity_transcript(),
        [
            "PHYSICAL front=15 back_attack=7 back_defend=7 both=3",
            "SPELL fire_bolt damage=12 mp=26",
            "ITEM potion hp=150 qty=4",
            "STATUS burn damage=4 hp=3 active=false",
            "BOSS flee=Blocked",
            "REWARD exp=7 awards=0,3,4 loot=mc_xs:2,rat_tail:1",
        ]
    );
}

#[test]
#[ignore = "requires RPG_S1_PINNED_SOURCE_DIR at the clean pinned Python source checkout"]
fn full_battle_parity_transcript_matches_the_pinned_python_oracle() {
    let source = pinned_python_source();
    assert_clean_pinned_python_source(&source);
    let python = source.join(".venv/bin/python");
    assert!(
        python.is_file(),
        "pinned source virtualenv Python is missing"
    );
    let output = std::process::Command::new(python)
        .args(["-c", PYTHON_BATTLE_PARITY_ORACLE])
        .current_dir(&source)
        .env("PYGAME_HIDE_SUPPORT_PROMPT", "1")
        .output()
        .expect("pinned Python battle oracle should run");
    assert!(
        output.status.success(),
        "pinned Python battle oracle failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        full_battle_parity_transcript()
    );
    assert_clean_pinned_python_source(&source);
}

#[test]
fn reward_application_rolls_back_every_change_when_a_later_member_is_invalid() {
    let (mut game, balance) = reward_game();
    let catalog = FieldMenuCatalog::production_class_fixture();
    let mut aric = actor(BattleSide::Party, 0, 10, 22);
    aric.id = "aric".to_owned();
    aric.class_id = "hero".to_owned();
    let mut missing = actor(BattleSide::Party, 1, 9, 10);
    missing.id = "not_in_runtime_party".to_owned();
    missing.class_id = "hero".to_owned();
    let mut enemy = reward_enemy("goblin", 0);
    enemy.experience_yield = 800;
    let mut state = state_with(vec![aric, missing, enemy]);
    let before = game.clone();

    assert!(matches!(
        apply_rewards(&mut state, &mut game, &catalog, &balance, None),
        Err(RewardError::MissingPartyMember(id)) if id == "not_in_runtime_party"
    ));
    assert_eq!(game, before);
    assert_eq!(state.rewards, None);
}

#[test]
fn enemy_target_selection_handles_single_wrap_cancel_and_no_target() {
    let actors = vec![
        actor(BattleSide::Party, 0, 4, 10),
        actor(BattleSide::Enemy, 0, 4, 0),
        actor(BattleSide::Enemy, 1, 4, 10),
        actor(BattleSide::Enemy, 2, 4, 10),
    ];
    let mut selector = TargetSelector::new(TargetGroup::Enemy, &actors, false).unwrap();
    assert_eq!(selector.selected(), CombatantKey::enemy(1));
    selector.navigate(-1);
    assert_eq!(selector.selected(), CombatantKey::enemy(2));
    selector.navigate(1);
    assert_eq!(selector.selected(), CombatantKey::enemy(1));
    assert!(TargetSelector::new(TargetGroup::Enemy, &actors[..2], false).is_none());

    let one = vec![actor(BattleSide::Enemy, 0, 1, 1)];
    let mut single = TargetSelector::new(TargetGroup::Enemy, &one, false).unwrap();
    single.navigate(1);
    assert_eq!(single.selected(), CombatantKey::enemy(0));
}

#[test]
fn ally_targeting_distinguishes_living_and_ko_eligibility() {
    let actors = vec![
        actor(BattleSide::Party, 0, 3, 10),
        actor(BattleSide::Party, 1, 2, 0),
        actor(BattleSide::Enemy, 0, 1, 10),
    ];
    assert_eq!(
        TargetSelector::new(TargetGroup::Ally, &actors, false)
            .unwrap()
            .selected(),
        CombatantKey::party(0)
    );
    assert_eq!(
        TargetSelector::new(TargetGroup::Ally, &actors, true)
            .unwrap()
            .selected(),
        CombatantKey::party(1)
    );
}

#[test]
fn turn_order_uses_dexterity_then_party_then_authored_order() {
    let actors = vec![
        actor(BattleSide::Enemy, 0, 8, 10),
        actor(BattleSide::Party, 0, 8, 10),
        actor(BattleSide::Party, 1, 12, 10),
        actor(BattleSide::Enemy, 1, 99, 0),
    ];
    assert_eq!(
        calculate_turn_order(&actors),
        vec![
            CombatantKey::party(1),
            CombatantKey::party(0),
            CombatantKey::enemy(0)
        ]
    );
}

#[test]
fn hit_chance_clamps_at_both_boundaries_and_seeded_rolls_repeat() {
    assert_eq!(physical_hit_chance(100, 1), 0.95);
    assert_eq!(physical_hit_chance(1, 100), 0.05);
    assert_eq!(physical_hit_chance(20, 20), 0.70);
    let mut left = GameplayRng::from_seed(77);
    let mut right = GameplayRng::from_seed(77);
    let left = (0..16)
        .map(|_| roll_succeeds(&mut left, 0.7))
        .collect::<Vec<_>>();
    let right = (0..16)
        .map(|_| roll_succeeds(&mut right, 0.7))
        .collect::<Vec<_>>();
    assert_eq!(left, right);
}

#[test]
fn flee_rate_uses_rogue_dex_caps_and_boss_or_failure_cost_rules() {
    let balance = BalanceData::default();
    let plain = state_with(vec![
        actor(BattleSide::Party, 0, 8, 10),
        actor(BattleSide::Enemy, 0, 4, 10),
    ]);
    assert_eq!(flee_chance(&plain, &balance), 0.30);

    let mut rogue = actor(BattleSide::Party, 0, 35, 10);
    rogue.class_id = "rogue".to_owned();
    let guaranteed = state_with(vec![rogue, actor(BattleSide::Enemy, 0, 4, 10)]);
    assert_eq!(flee_chance(&guaranteed, &balance), 1.0);

    let mut rng = GameplayRng::from_seed(5);
    let before = rng.state();
    assert_eq!(roll_flee(false, 1.0, &mut rng), FleeOutcome::Success);
    assert_eq!(
        rng.state(),
        before,
        "guaranteed flee consumes no random roll"
    );
    assert_eq!(roll_flee(true, 1.0, &mut rng), FleeOutcome::Blocked);
    assert_eq!(
        rng.state(),
        before,
        "boss restriction consumes no random roll"
    );

    assert_eq!(
        phase_after_flee_confirmation(FleeOutcome::Failed),
        Some(BattlePhase::Advance)
    );
    assert_eq!(
        phase_after_flee_confirmation(FleeOutcome::Blocked),
        Some(BattlePhase::Advance)
    );
    assert_eq!(phase_after_flee_confirmation(FleeOutcome::Success), None);
}

#[test]
fn physical_damage_applies_attack_defense_rows_minimum_and_health_cap() {
    let mut front = actor(BattleSide::Party, 0, 1, 99);
    front.attack = 20;
    let mut enemy = actor(BattleSide::Enemy, 0, 1, 50);
    enemy.defense = 6;
    assert_eq!(physical_damage(&front, &enemy), 14);
    front.row = PartyRow::Back;
    assert_eq!(physical_damage(&front, &enemy), 7);
    enemy.health = 3;
    assert_eq!(physical_damage(&front, &enemy), 3);
    front.attack = 1;
    enemy.defense = 999;
    enemy.health = 50;
    assert_eq!(physical_damage(&front, &enemy), 1);
    let mut back_party = actor(BattleSide::Party, 1, 1, 50);
    back_party.row = PartyRow::Back;
    let enemy_attacker = actor(BattleSide::Enemy, 0, 1, 50);
    assert_eq!(physical_damage(&enemy_attacker, &back_party), 3);
}

#[test]
fn resolving_an_action_is_pure_until_its_typed_event_is_applied() {
    let mut state = state_with(vec![
        actor(BattleSide::Party, 0, 100, 20),
        actor(BattleSide::Enemy, 0, 1, 20),
    ]);
    let action = BattleAction::Physical {
        attacker: CombatantKey::party(0),
        target: CombatantKey::enemy(0),
    };
    let before = state.combatants[1].health;
    let event = resolve_action(&state, action, &mut GameplayRng::from_seed(9)).unwrap();

    assert_eq!(state.combatants[1].health, before);
    assert_eq!(
        event,
        BattleEvent::Damage {
            action,
            amount: 7,
            critical: false,
            knocked_out: false,
        }
    );

    state.apply_event(event);
    assert_eq!(state.combatants[1].health, before - 7);
    assert_eq!(state.phase, BattlePhase::Resolve);
}

#[test]
fn critical_chance_and_damage_match_the_source_formula_boundaries() {
    assert_eq!(critical_hit_chance(-1), 0.0);
    assert_eq!(critical_hit_chance(1), 0.02);
    assert_eq!(critical_hit_chance(10), 0.20);
    assert_eq!(critical_hit_chance(13), 0.25);
    assert_eq!(critical_hit_chance(100), 0.25);
    assert_eq!(critical_damage(1), 1);
    assert_eq!(critical_damage(7), 10);
    assert_eq!(critical_damage(20), 30);
}

#[test]
fn seeded_physical_resolution_emits_distinct_miss_and_critical_feedback() {
    let mut critical_state = state_with(vec![
        actor(BattleSide::Party, 0, 100, 20),
        actor(BattleSide::Enemy, 0, 1, 20),
    ]);
    let action = BattleAction::Physical {
        attacker: CombatantKey::party(0),
        target: CombatantKey::enemy(0),
    };
    let event = resolve_action(&critical_state, action, &mut GameplayRng::from_seed(7)).unwrap();
    assert_eq!(
        event,
        BattleEvent::Damage {
            action,
            amount: 10,
            critical: true,
            knocked_out: false,
        }
    );
    critical_state.apply_event(event);
    assert!(critical_state.message.starts_with("Critical hit!"));
    assert_eq!(
        critical_state.transcript.last().map(String::as_str),
        Some("CRITICAL Party-0 -> Enemy-0 10")
    );

    let miss_state = state_with(vec![
        actor(BattleSide::Party, 0, 1, 20),
        actor(BattleSide::Enemy, 0, 100, 20),
    ]);
    assert_eq!(
        resolve_action(&miss_state, action, &mut GameplayRng::from_seed(0)),
        Some(BattleEvent::Miss { action })
    );
}

#[test]
fn damage_clamps_to_zero_and_turn_advance_skips_ko_actors() {
    let mut target = actor(BattleSide::Party, 0, 1, 4);
    assert_eq!(target.apply_damage(99), 4);
    assert!(!target.is_alive());

    let mut state = BattleState {
        phase: BattlePhase::Advance,
        combatants: vec![
            target,
            actor(BattleSide::Party, 1, 3, 10),
            actor(BattleSide::Enemy, 0, 2, 10),
        ],
        turn_order: vec![
            CombatantKey::party(0),
            CombatantKey::party(1),
            CombatantKey::enemy(0),
        ],
        active_turn: 0,
        turn_count: 1,
        command_index: 0,
        ability_index: 0,
        pending_ability: None,
        item_index: 0,
        item_choices: Vec::new(),
        pending_item: None,
        target: None,
        message: String::new(),
        transcript: Vec::new(),
        feedback_events: Vec::new(),
        used_enemy_moves: std::collections::HashSet::new(),
        rewards: None,
        flee_outcome: None,
    };
    state.skip_knocked_out();
    assert_eq!(state.active_key(), Some(CombatantKey::party(1)));
}

#[test]
fn victory_and_defeat_require_every_member_of_one_side_to_be_ko() {
    let mut state = BattleState {
        phase: BattlePhase::Resolve,
        combatants: vec![
            actor(BattleSide::Party, 0, 2, 10),
            actor(BattleSide::Enemy, 0, 1, 0),
            actor(BattleSide::Enemy, 1, 1, 1),
        ],
        turn_order: vec![],
        active_turn: 0,
        turn_count: 1,
        command_index: 0,
        ability_index: 0,
        pending_ability: None,
        item_index: 0,
        item_choices: Vec::new(),
        pending_item: None,
        target: None,
        message: String::new(),
        transcript: vec![],
        feedback_events: Vec::new(),
        used_enemy_moves: std::collections::HashSet::new(),
        rewards: None,
        flee_outcome: None,
    };
    state.assess_result();
    assert_eq!(state.phase, BattlePhase::Advance);
    state.combatants[2].health = 0;
    state.assess_result();
    assert_eq!(state.phase, BattlePhase::Victory);
    state.combatants[0].health = 0;
    state.combatants[2].health = 1;
    state.assess_result();
    assert_eq!(state.phase, BattlePhase::Defeat);
}

#[test]
fn deterministic_basic_battle_transcript_is_stable() {
    fn replay(seed: u64) -> Vec<String> {
        let mut state = BattleState {
            phase: BattlePhase::Command,
            combatants: vec![
                actor(BattleSide::Party, 0, 8, 40),
                actor(BattleSide::Enemy, 0, 4, 16),
            ],
            turn_order: vec![CombatantKey::party(0), CombatantKey::enemy(0)],
            active_turn: 0,
            turn_count: 1,
            command_index: 0,
            ability_index: 0,
            pending_ability: None,
            item_index: 0,
            item_choices: Vec::new(),
            pending_item: None,
            target: None,
            message: String::new(),
            transcript: vec!["START fixture".to_owned()],
            feedback_events: Vec::new(),
            used_enemy_moves: std::collections::HashSet::new(),
            rewards: None,
            flee_outcome: None,
        };
        state.combatants[0].attack = 14;
        state.combatants[1].defense = 2;
        let mut rng = GameplayRng::from_seed(seed);
        while !state.all_defeated(BattleSide::Enemy) {
            if state.active_key() == Some(CombatantKey::party(0)) {
                state.resolve_physical(CombatantKey::party(0), CombatantKey::enemy(0), &mut rng);
            } else {
                state.resolve_enemy_action(&mut rng);
            }
            state.assess_result();
            if state.phase == BattlePhase::Advance {
                state.advance(&mut rng);
            }
        }
        state.transcript
    }
    let transcript = replay(9);
    assert_eq!(transcript, replay(9));
    assert_eq!(transcript.last().map(String::as_str), Some("VICTORY"));
}
