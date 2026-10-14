//! Party, class, enemy, enemy AI, enemy sprite, and encounter references.

use std::collections::BTreeSet;

use super::{
    context::{Validator, file_stem, probe_path},
    index::ReferenceIndex,
};
use crate::{
    manifest_path_validation::ScenarioPathProbeResult,
    scenario_enemy::{EnemyAi, EnemyBehavior, EnemyMove, EnemyTargetOverride},
    scenario_path::ScenarioRelativePath,
};

impl<'a> Validator<'a> {
    pub(super) fn validate_party(&mut self, index: &ReferenceIndex) {
        let Some(file) = self.catalogs.party.clone() else {
            return;
        };
        for (member_index, member) in file.value.party.iter().enumerate() {
            let data = member.data();
            let base = format!("party[{member_index}]");
            self.checked(
                "class",
                &index.classes,
                &data.class_id,
                &file.path,
                format!("{base}.class"),
            );
            self.checked(
                "map",
                &index.maps,
                &data.join.map,
                &file.path,
                format!("{base}.join.map"),
            );
            self.checked_path(
                &file.path,
                format!("{base}.portrait"),
                data.portrait.as_str(),
            );
            for (slot, namespace, known, id) in [
                ("weapon", "weapon", &index.weapons, &data.equipped.weapon),
                ("shield", "shield", &index.shields, &data.equipped.shield),
                ("helmet", "helmet", &index.helmets, &data.equipped.helmet),
                ("body", "body armor", &index.bodies, &data.equipped.body),
                (
                    "accessory",
                    "accessory",
                    &index.accessories,
                    &data.equipped.accessory,
                ),
            ] {
                self.checked(
                    namespace,
                    known,
                    id,
                    &file.path,
                    format!("{base}.equipped.{slot}"),
                );
            }
            let no_abilities = BTreeSet::new();
            let class_abilities = index
                .class_abilities
                .get(&data.class_id)
                .unwrap_or(&no_abilities);
            for (ability_index, ability) in data.abilities_unlocked.iter().enumerate() {
                self.checked(
                    "ability owned by the member class",
                    class_abilities,
                    ability,
                    &file.path,
                    format!("{base}.abilities_unlocked[{ability_index}]"),
                );
            }
            if let Some(recruit) = member.recruit() {
                self.checked(
                    "dialogue",
                    &index.dialogue,
                    &recruit.dialogue,
                    &file.path,
                    format!("{base}.recruit.dialogue"),
                );
                self.report.checked_references += 1;
                if !index
                    .npcs
                    .get(&data.join.map)
                    .is_some_and(|ids| ids.contains(&recruit.npc))
                {
                    self.error(
                        "reference.missing",
                        &file.path,
                        format!("{base}.recruit.npc"),
                        format!(
                            "unknown NPC id `{}` on join map `{}`",
                            recruit.npc, data.join.map
                        ),
                    );
                }
            }
        }
    }

    pub(super) fn validate_enemies(&mut self, index: &ReferenceIndex) {
        let enemy_files = self.catalogs.enemies.clone();
        for file in enemy_files {
            for (enemy_index, enemy) in file.value.entries().iter().enumerate() {
                let base = format!("documents[{enemy_index}]");
                if let Some(barrier) = &enemy.barrier {
                    self.checked(
                        "item",
                        &index.items,
                        &barrier.requires_item,
                        &file.path,
                        format!("{base}.requires_item"),
                    );
                }
                for (pool_index, pool) in enemy.drops.loot.iter().enumerate() {
                    for (item_index, item) in pool.pool.iter().enumerate() {
                        self.checked(
                            "item",
                            &index.items,
                            &item.item,
                            &file.path,
                            format!("{base}.drops.loot[{pool_index}].pool[{item_index}].item"),
                        );
                    }
                }
                match &enemy.behavior {
                    EnemyBehavior::Inline { ai, targeting } => self.validate_enemy_ai(
                        ai,
                        &targeting.overrides,
                        &file.path,
                        &format!("{base}.ai"),
                        index,
                    ),
                    EnemyBehavior::Referenced { ai_ref } => {
                        let referenced = format!("{}/{}", index.enemy_root, ai_ref.as_str());
                        self.checked_path(&file.path, format!("{base}.ai_ref"), &referenced);
                        self.checked(
                            "boss move set",
                            &index.boss_move_sets,
                            file_stem(ai_ref.as_str()),
                            &file.path,
                            format!("{base}.ai_ref"),
                        );
                    }
                }
                self.validate_enemy_sprite(&file.path, &base, enemy.sprite_id());
            }
        }
        let boss_files = self.catalogs.boss_move_sets.clone();
        for file in boss_files {
            self.validate_enemy_ai(
                &file.value.ai,
                &file.value.targeting.overrides,
                &file.path,
                "ai",
                index,
            );
        }
    }

    fn validate_enemy_ai(
        &mut self,
        ai: &EnemyAi,
        overrides: &[EnemyTargetOverride],
        path: &str,
        base: &str,
        index: &ReferenceIndex,
    ) {
        let local_abilities = ai
            .moves
            .iter()
            .filter_map(|move_| match move_ {
                EnemyMove::Ability { id, .. } => Some(id.as_str()),
                EnemyMove::Attack { .. } => None,
            })
            .collect::<BTreeSet<_>>();
        for (override_index, override_) in overrides.iter().enumerate() {
            let ability = match override_ {
                EnemyTargetOverride::Standard(value) => &value.ability,
                EnemyTargetOverride::AccessoryBlocked(value) => {
                    self.checked(
                        "accessory",
                        &index.accessories,
                        &value.blocked_by_accessory,
                        path,
                        format!(
                            "{base}.targeting.overrides[{override_index}].blocked_by_accessory"
                        ),
                    );
                    &value.ability
                }
            };
            self.report.checked_references += 1;
            if !local_abilities.contains(ability.as_str()) {
                self.error(
                    "reference.missing",
                    path,
                    format!("{base}.targeting.overrides[{override_index}].ability"),
                    format!(
                        "target override names ability `{ability}` absent from this AI move set"
                    ),
                );
            }
        }
    }

    fn validate_enemy_sprite(&mut self, owner_path: &str, base: &str, enemy_id: &str) {
        self.report.checked_references += 1;
        let ordinary = format!("media/sprites/enemies/{enemy_id}.tsx");
        let battle = format!("media/sprites/enemies/{enemy_id}_battle.tsx");
        let exists = [ordinary.as_str(), battle.as_str()]
            .iter()
            .any(|candidate| {
                ScenarioRelativePath::try_from(*candidate).is_ok_and(|path| {
                    probe_path(self.physical_root, self.canonical_root.as_deref(), &path)
                        == ScenarioPathProbeResult::File
                })
            });
        if !exists {
            self.error(
                "path.missing",
                owner_path,
                format!("{base}.$derived_sprite"),
                format!("enemy `{enemy_id}` has neither `{ordinary}` nor `{battle}`"),
            );
        }
    }

    pub(super) fn validate_encounters(&mut self, index: &ReferenceIndex) {
        let encounters = self.catalogs.encounters.clone();
        for file in encounters {
            self.checked(
                "map",
                &index.maps,
                file.value.effective_id(&file.stem),
                &file.path,
                "id",
            );
            if !file.value.background.is_empty() {
                self.checked(
                    "battle background",
                    &index.backgrounds,
                    &file.value.background,
                    &file.path,
                    "background",
                );
            }
            for (formation_index, formation) in file.value.entries.iter().enumerate() {
                for (enemy_index, enemy) in formation.enemy_ids.iter().enumerate() {
                    self.checked(
                        "enemy",
                        &index.enemies,
                        enemy,
                        &file.path,
                        format!("entries[{formation_index}].formation[{enemy_index}]"),
                    );
                }
            }
            if let Some(boss) = &file.value.boss {
                self.checked(
                    "boss enemy",
                    &index.boss_enemies,
                    &boss.enemy_id,
                    &file.path,
                    "boss.id",
                );
            }
            for (barrier_index, barrier) in file.value.barrier_enemies.iter().enumerate() {
                self.checked(
                    "enemy",
                    &index.enemies,
                    &barrier.enemy_id,
                    &file.path,
                    format!("barrier_enemies[{barrier_index}].id"),
                );
                self.checked(
                    "item",
                    &index.items,
                    &barrier.requires_item,
                    &file.path,
                    format!("barrier_enemies[{barrier_index}].requires_item"),
                );
            }
        }
        let encounter_ids = self
            .catalogs
            .encounters
            .iter()
            .map(|file| file.value.effective_id(&file.stem).to_owned())
            .collect::<BTreeSet<_>>();
        let maps = self.catalogs.maps.clone();
        for file in maps {
            if file.value.enemy_spawn.is_some()
                && index.maps.contains(file.value.effective_id(&file.stem))
            {
                self.checked(
                    "encounter",
                    &encounter_ids,
                    file.value.effective_id(&file.stem),
                    &file.path,
                    "enemy_spawn",
                );
            }
        }
        if let Some(backgrounds) = self.catalogs.backgrounds.clone() {
            for (background_index, background) in backgrounds.value.0.iter().enumerate() {
                let asset = format!("media/images/battle_bg/{}.webp", background.id);
                self.checked_path(
                    &backgrounds.path,
                    format!("[{background_index}].$derived_image"),
                    &asset,
                );
            }
        }
    }
}
