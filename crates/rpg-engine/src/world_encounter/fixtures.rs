//! Session-only debug battle fixtures that isolate one manual observation each.

use bevy::prelude::*;

use crate::{
    debug_launch::DebugBattleFixture,
    encounter::{BattleEntry, BattleParticipant, BattleSide},
    field_menu_domain::FieldMenuCatalog,
    scenario_class::{Ability, AbilityKind, AbilitySideEffect, UnitInterval},
    scenario_enemy::{EnemyAiPattern, EnemyBehavior, EnemyMove},
    scenario_party::PartyRow,
};

pub(super) fn apply_debug_battle_fixture(
    entry: &mut BattleEntry,
    fixture: DebugBattleFixture,
    catalog: &FieldMenuCatalog,
) -> Result<(), String> {
    match fixture {
        DebugBattleFixture::Feedback => {
            retain_fixture_participants(entry, &["aric", "elise", "jep"]);
            let power_strike = fixture_ability(catalog, "hero", "power_strike")?;
            let shadow_step = fixture_ability(catalog, "rogue", "shadow_step")?;
            for id in ["aric", "elise", "jep"] {
                let actor = fixture_participant(entry, id)?;
                actor.attack = 30;
                actor.defense = 40;
                // A 5% capped basic hit chance makes MISS feedback immediate, while the two
                // injected physical abilities provide deterministic normal/critical hits.
                actor.dexterity = 0;
                actor.health = 250;
                actor.max_health = 250;
                actor.mana = 99;
                actor.max_mana = 99;
            }
            fixture_participant(entry, "aric")?.row = PartyRow::Front;
            fixture_participant(entry, "elise")?.row = PartyRow::Back;
            fixture_participant(entry, "aric")?.abilities = vec![power_strike.clone()];
            fixture_participant(entry, "elise")?.abilities = vec![power_strike];
            fixture_participant(entry, "jep")?.abilities = vec![shadow_step];
            let enemy = fixture_enemy(entry)?;
            enemy.health = 600;
            enemy.max_health = 600;
            enemy.attack = 1;
            enemy.defense = 10;
            enemy.dexterity = 100;
            enemy.behavior = None;
            enemy.experience_yield = 0;
            enemy.gold_yield = 0;
            enemy.drops = None;
        }
        DebugBattleFixture::Status => {
            retain_fixture_participants(entry, &["aric", "reiya"]);
            let rally = fixture_ability(catalog, "hero", "rally")?;
            let war_cry = fixture_ability(catalog, "hero", "war_cry")?;
            let mut boulder_crash = fixture_ability(catalog, "sorcerer", "boulder_crash")?;
            let AbilityKind::Spell(spell) = &mut boulder_crash.kind else {
                return Err("status fixture expected Boulder Crash to be a spell".to_owned());
            };
            for side_effect in &mut spell.side_effects {
                if let AbilitySideEffect::Stun { chance, .. } = side_effect {
                    *chance = UnitInterval::new(1.0).expect("one is a valid unit interval");
                }
            }
            if !spell
                .side_effects
                .iter()
                .any(|effect| matches!(effect, AbilitySideEffect::Stun { .. }))
            {
                return Err("status fixture expected Boulder Crash to carry stun".to_owned());
            }
            let aric = fixture_participant(entry, "aric")?;
            aric.abilities = vec![rally, war_cry];
            aric.mana = 99;
            aric.max_mana = 99;
            aric.health = 300;
            aric.max_health = 300;
            aric.dexterity = 30;
            let reiya = fixture_participant(entry, "reiya")?;
            reiya.abilities = vec![boulder_crash];
            reiya.mana = 99;
            reiya.max_mana = 99;
            reiya.health = 250;
            reiya.max_health = 250;
            reiya.dexterity = 25;
            let enemy = fixture_enemy(entry)?;
            enemy.health = 900;
            enemy.max_health = 900;
            enemy.attack = 1;
            enemy.defense = 40;
            enemy.magic_resistance = 40;
            enemy.dexterity = 1;
            let Some(EnemyBehavior::Inline { ai, .. }) = enemy.behavior.as_mut() else {
                return Err("status fixture expected resolved Troll Sage AI".to_owned());
            };
            ai.pattern = EnemyAiPattern::Random;
            ai.moves.retain(
                |movement| matches!(movement, EnemyMove::Ability { id, .. } if id == "charm"),
            );
            if ai.moves.len() != 1 {
                return Err("status fixture could not isolate the Troll Sage Charm move".to_owned());
            }
            enemy.experience_yield = 0;
            enemy.gold_yield = 0;
            enemy.drops = None;
        }
        DebugBattleFixture::EnemyAi => {
            retain_fixture_participants(entry, &["aric"]);
            let rally = fixture_ability(catalog, "hero", "rally")?;
            let aric = fixture_participant(entry, "aric")?;
            aric.health = 500;
            aric.max_health = 500;
            aric.attack = 1;
            aric.defense = 60;
            aric.mana = 99;
            aric.max_mana = 99;
            aric.abilities = vec![rally];
            let enemy = fixture_enemy(entry)?;
            enemy.health = 900;
            enemy.max_health = 900;
            enemy.attack = 10;
            enemy.defense = 60;
            enemy.experience_yield = 0;
            enemy.gold_yield = 0;
            enemy.drops = None;
        }
        DebugBattleFixture::Rewards => {
            retain_fixture_participants(entry, &["aric"]);
            let aric = fixture_participant(entry, "aric")?;
            aric.attack = 100;
            aric.dexterity = 100;
            let enemy = fixture_enemy(entry)?;
            enemy.health = 1;
            enemy.max_health = 1;
            enemy.attack = 1;
            enemy.defense = 1;
            enemy.dexterity = 1;
            enemy.experience_yield = 1_000;
            enemy.gold_yield = 75;
            if enemy
                .drops
                .as_ref()
                .is_none_or(|drops| drops.mc.is_empty() && drops.loot.is_empty())
            {
                return Err("rewards fixture requires the Ashen Crown's authored loot".to_owned());
            }
        }
    }
    Ok(())
}

pub(super) fn retain_fixture_participants(entry: &mut BattleEntry, party_ids: &[&str]) {
    entry.participants.retain(|actor| {
        actor.side == BattleSide::Enemy
            || (actor.side == BattleSide::Party && party_ids.contains(&actor.id.as_str()))
    });
}

pub(super) fn fixture_participant<'a>(
    entry: &'a mut BattleEntry,
    id: &str,
) -> Result<&'a mut BattleParticipant, String> {
    entry
        .participants
        .iter_mut()
        .find(|actor| actor.side == BattleSide::Party && actor.id == id)
        .ok_or_else(|| format!("battle fixture requires party member `{id}`"))
}

pub(super) fn fixture_enemy(entry: &mut BattleEntry) -> Result<&mut BattleParticipant, String> {
    entry
        .participants
        .iter_mut()
        .find(|actor| actor.side == BattleSide::Enemy)
        .ok_or_else(|| "battle fixture requires one enemy".to_owned())
}

pub(super) fn fixture_ability(
    catalog: &FieldMenuCatalog,
    class_id: &str,
    ability_id: &str,
) -> Result<Ability, String> {
    catalog
        .class(class_id)
        .and_then(|class| {
            class
                .abilities
                .iter()
                .find(|ability| ability.id == ability_id)
        })
        .cloned()
        .ok_or_else(|| format!("battle fixture requires ability `{ability_id}`"))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::{
        encounter::PreBattleReturnContext,
        scenario_enemy::BossMoveSet,
        scenario_spatial::{CardinalDirection, Position},
    };

    fn fixture_actor(side: BattleSide, id: &str, row: PartyRow) -> BattleParticipant {
        BattleParticipant {
            side,
            id: id.to_owned(),
            name: id.to_owned(),
            class_id: match id {
                "aric" => "hero",
                "elise" => "cleric",
                "reiya" => "sorcerer",
                "jep" => "rogue",
                _ => "",
            }
            .to_owned(),
            health: 20,
            max_health: 20,
            mana: 20,
            max_mana: 20,
            attack: 20,
            defense: 5,
            magic_resistance: 5,
            dexterity: 10,
            abilities: Vec::new(),
            status_effects: Vec::new(),
            accessory: None,
            wields_sword: false,
            row,
            boss: side == BattleSide::Enemy,
            enemy_type: None,
            immunities: Vec::new(),
            behavior: None,
            experience_yield: 10,
            gold_yield: 2,
            enemy_size: None,
            sprite_id: id.to_owned(),
            sprite_scale_percent: 100,
            drops: None,
        }
    }

    fn fixture_battle_entry(enemy_id: &str) -> BattleEntry {
        let mut enemy = fixture_actor(BattleSide::Enemy, enemy_id, PartyRow::Front);
        let move_set: BossMoveSet = crate::scenario_yaml::from_str(include_str!(scenario_file!(
            "data/enemies/boss_move_sets/troll_shaman_base.yaml"
        )))
        .unwrap();
        enemy.behavior = Some(EnemyBehavior::Inline {
            ai: move_set.ai,
            targeting: move_set.targeting,
        });
        enemy.drops = Some(crate::scenario_enemy::EnemyDrops {
            mc: vec![crate::scenario_enemy::MagicCoreDrop {
                size: crate::scenario_enemy::MagicCoreSize::ExtraLarge,
                qty: NonZeroU32::new(3).unwrap(),
            }],
            loot: vec![crate::scenario_enemy::EnemyLootPool {
                pool: vec![crate::scenario_enemy::EnemyLootEntry {
                    item: "void_core".to_owned(),
                    weight: NonZeroU32::new(100).unwrap(),
                }],
            }],
        });
        BattleEntry {
            encounter_id: "fixture".to_owned(),
            participants: vec![
                fixture_actor(BattleSide::Party, "aric", PartyRow::Front),
                fixture_actor(BattleSide::Party, "elise", PartyRow::Back),
                fixture_actor(BattleSide::Party, "reiya", PartyRow::Back),
                fixture_actor(BattleSide::Party, "jep", PartyRow::Back),
                enemy,
            ],
            background_id: "fixture".to_owned(),
            background_asset: "fixture.webp".to_owned(),
            bgm_key: "battle.normal".to_owned(),
            boss_completion_flag: None,
            victory_flags: Vec::new(),
            barrier_messages: Vec::new(),
            return_context: PreBattleReturnContext {
                map_id: "zone_01_starting_forest".to_owned(),
                position: Position::new(29, 1),
                facing: CardinalDirection::Down,
                world_bgm_key: None,
                world_enemies: Vec::new(),
            },
        }
    }

    #[test]
    fn manual_battle_fixtures_isolate_the_named_observations() {
        let catalog = FieldMenuCatalog::production_class_fixture();

        let mut feedback = fixture_battle_entry("goblin_warrior");
        apply_debug_battle_fixture(&mut feedback, DebugBattleFixture::Feedback, &catalog).unwrap();
        assert_eq!(feedback.participants.len(), 4);
        let aric = feedback
            .participants
            .iter()
            .find(|actor| actor.id == "aric")
            .unwrap();
        let elise = feedback
            .participants
            .iter()
            .find(|actor| actor.id == "elise")
            .unwrap();
        assert_eq!((aric.attack, aric.dexterity), (30, 0));
        assert_eq!((elise.attack, elise.dexterity), (30, 0));
        assert_eq!((aric.row, elise.row), (PartyRow::Front, PartyRow::Back));
        assert_eq!(aric.abilities[0].id, "power_strike");
        assert_eq!(elise.abilities[0].id, "power_strike");
        let jep = feedback
            .participants
            .iter()
            .find(|actor| actor.id == "jep")
            .unwrap();
        assert_eq!(jep.abilities[0].id, "shadow_step");

        let mut status = fixture_battle_entry("troll_shaman_base");
        apply_debug_battle_fixture(&mut status, DebugBattleFixture::Status, &catalog).unwrap();
        assert_eq!(status.participants.len(), 3);
        let aric = status
            .participants
            .iter()
            .find(|actor| actor.id == "aric")
            .unwrap();
        assert_eq!(
            aric.abilities
                .iter()
                .map(|ability| ability.id.as_str())
                .collect::<Vec<_>>(),
            ["rally", "war_cry"]
        );
        let reiya = status
            .participants
            .iter()
            .find(|actor| actor.id == "reiya")
            .unwrap();
        let AbilityKind::Spell(spell) = &reiya.abilities[0].kind else {
            unreachable!()
        };
        assert!(spell.side_effects.iter().any(|effect| matches!(
            effect,
            AbilitySideEffect::Stun { chance, .. } if chance.get() == 1.0
        )));
        let enemy = status
            .participants
            .iter()
            .find(|actor| actor.side == BattleSide::Enemy)
            .unwrap();
        let Some(EnemyBehavior::Inline { ai, .. }) = &enemy.behavior else {
            unreachable!()
        };
        assert!(matches!(
            ai.moves.as_slice(),
            [EnemyMove::Ability { id, .. }] if id == "charm"
        ));

        let mut enemy_ai = fixture_battle_entry("troll_shaman_base");
        apply_debug_battle_fixture(&mut enemy_ai, DebugBattleFixture::EnemyAi, &catalog).unwrap();
        assert_eq!(enemy_ai.participants.len(), 2);
        let aric = enemy_ai
            .participants
            .iter()
            .find(|actor| actor.id == "aric")
            .unwrap();
        assert_eq!(
            aric.abilities
                .iter()
                .map(|ability| ability.id.as_str())
                .collect::<Vec<_>>(),
            ["rally"]
        );
        let enemy = enemy_ai
            .participants
            .iter()
            .find(|actor| actor.side == BattleSide::Enemy)
            .unwrap();
        assert_eq!(enemy.health, enemy.max_health);

        let mut rewards = fixture_battle_entry("hearth_effigy_ashen_crown");
        apply_debug_battle_fixture(&mut rewards, DebugBattleFixture::Rewards, &catalog).unwrap();
        assert_eq!(rewards.participants.len(), 2);
        let enemy = rewards
            .participants
            .iter()
            .find(|actor| actor.side == BattleSide::Enemy)
            .unwrap();
        assert_eq!(
            (enemy.health, enemy.experience_yield, enemy.gold_yield),
            (1, 1_000, 75)
        );
        assert!(
            enemy
                .drops
                .as_ref()
                .is_some_and(|drops| !drops.mc.is_empty() && !drops.loot.is_empty())
        );
    }
}
