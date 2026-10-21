//! Dialogue actions, flag producers and consumers, and the quest flag edges.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    context::{ScenarioCatalogs, Validator},
    index::ReferenceIndex,
    report::{DiagnosticSeverity, ScenarioDiagnostic, ScenarioLocation},
};
use crate::{
    scenario_condition::FlagConditions,
    scenario_dialogue::{DialogueActions, DialogueDocument},
    scenario_item::ItemDefinition,
    scenario_root::SCENARIO_MANIFEST_PATH,
};

/// Which document kind an `on_complete` / `on_select` block was authored in.
///
/// Only `start_battle` cares: the source documents it as `npc`-only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DialogueActionContext {
    Cutscene,
    Entries,
}

type FlagLocations = BTreeMap<String, Vec<ScenarioLocation>>;

fn collect_flag_edges(
    catalogs: &ScenarioCatalogs,
) -> (FlagLocations, FlagLocations, BTreeSet<String>) {
    let mut defined = FlagLocations::new();
    let mut consumed = FlagLocations::new();
    let mut engine_managed = BTreeSet::new();
    let add = |map: &mut FlagLocations, flag: &str, path: &str, field: String| {
        if !flag.is_empty() {
            map.entry(flag.to_owned())
                .or_default()
                .push(ScenarioLocation::new(path, field));
        }
    };
    if let Some(manifest) = &catalogs.manifest {
        for (index, flag) in manifest.bootstrap_flags.iter().enumerate() {
            add(
                &mut defined,
                flag,
                SCENARIO_MANIFEST_PATH,
                format!("bootstrap_flags[{index}]"),
            );
        }
        for (index, flag) in manifest.engine_managed_flags.iter().enumerate() {
            add(
                &mut defined,
                flag,
                SCENARIO_MANIFEST_PATH,
                format!("engine_managed_flags[{index}]"),
            );
            engine_managed.insert(flag.clone());
        }
    }
    if let Some(file) = &catalogs.transport {
        add(
            &mut consumed,
            &file.value.sail_unlock_flag,
            &file.path,
            "sail_unlock_flag".to_owned(),
        );
        add(
            &mut consumed,
            &file.value.fly_unlock_flag,
            &file.path,
            "fly_unlock_flag".to_owned(),
        );
        for (kind, anchors) in [
            ("fly_anchors", &file.value.fly_anchors),
            ("sail_berths", &file.value.sail_berths),
        ] {
            for (index, anchor) in anchors.iter().enumerate() {
                if let Some(flag) = &anchor.reveal_flag {
                    add(
                        &mut consumed,
                        flag,
                        &file.path,
                        format!("{kind}[{index}].reveal_flag"),
                    );
                }
            }
        }
    }
    if let Some(file) = &catalogs.party {
        for (index, member) in file.value.party.iter().enumerate() {
            add(
                &mut consumed,
                &member.data().join.condition,
                &file.path,
                format!("party[{index}].join.condition"),
            );
            if let Some(recruit) = member.recruit() {
                add(
                    &mut consumed,
                    &recruit.joined_flag,
                    &file.path,
                    format!("party[{index}].recruit.joined_flag"),
                );
            }
        }
    }
    for file in &catalogs.classes {
        for (index, ability) in file.value.abilities.iter().enumerate() {
            if let Some(flag) = &ability.unlock_flag {
                add(
                    &mut consumed,
                    flag,
                    &file.path,
                    format!("abilities[{index}].unlock_flag"),
                );
            }
        }
    }
    for file in &catalogs.items {
        for (index, item) in file.value.entries().iter().enumerate() {
            if let ItemDefinition::Key(item) = item
                && let crate::scenario_item::KeyItemEffect::Unlock(effect) = &item.effect
            {
                add(
                    &mut defined,
                    &effect.unlock_flag,
                    &file.path,
                    format!("[{index}].effect.unlock_flag"),
                );
            }
        }
    }
    if let Some(file) = &catalogs.item_boxes {
        for (id, definition) in &file.value.item_boxes {
            add_conditions(
                &mut consumed,
                &definition.present,
                &file.path,
                format!("item_boxes.{id}.present"),
            );
        }
    }
    for file in &catalogs.maps {
        for (i, npc) in file.value.npcs.iter().enumerate() {
            add_conditions(
                &mut consumed,
                &npc.present,
                &file.path,
                format!("npcs[{i}].present"),
            );
        }
        for (i, b) in file.value.item_boxes.iter().enumerate() {
            add_conditions(
                &mut consumed,
                &b.present,
                &file.path,
                format!("item_boxes[{i}].present"),
            );
        }
        for (section, shop) in [
            ("shop", file.value.shop.as_ref()),
            ("weapon_shop", file.value.weapon_shop.as_ref()),
            ("armor_shop", file.value.armor_shop.as_ref()),
        ] {
            if let Some(shop) = shop {
                for (i, item) in shop.items.iter().enumerate() {
                    add(
                        &mut consumed,
                        item.unlock_flag(),
                        &file.path,
                        format!("{section}.items[{i}].unlock_flag"),
                    );
                }
            }
        }
    }
    for file in &catalogs.dialogue {
        match &file.value {
            DialogueDocument::Cutscene(d) => {
                add_actions(&mut defined, &d.on_complete, &file.path, "on_complete")
            }
            DialogueDocument::Entries(d) => {
                for (i, e) in d.entries.iter().enumerate() {
                    add_conditions(
                        &mut consumed,
                        &e.condition,
                        &file.path,
                        format!("entries[{i}].condition"),
                    );
                    add_actions(
                        &mut defined,
                        &e.on_complete,
                        &file.path,
                        &format!("entries[{i}].on_complete"),
                    );
                    // Choices carry their own conditions and effects, so a flag can be both
                    // consumed and produced without ever appearing on an entry.
                    for (j, c) in e.choices.iter().enumerate() {
                        add_conditions(
                            &mut consumed,
                            &c.condition,
                            &file.path,
                            format!("entries[{i}].choices[{j}].condition"),
                        );
                        add_conditions(
                            &mut consumed,
                            &c.enabled,
                            &file.path,
                            format!("entries[{i}].choices[{j}].enabled"),
                        );
                        add_actions(
                            &mut defined,
                            &c.on_select,
                            &file.path,
                            &format!("entries[{i}].choices[{j}].on_select"),
                        );
                    }
                }
            }
            DialogueDocument::LinePool(_) => {}
        }
    }
    for file in &catalogs.encounters {
        if let Some(b) = &file.value.boss {
            add(
                &mut defined,
                &b.completion.set_flag,
                &file.path,
                "boss.on_complete.set_flag".to_owned(),
            );
        }
    }
    for file in &catalogs.recipes {
        for (i, r) in file.value.entries().iter().enumerate() {
            if let Some(flag) = &r.unlock_flag {
                add(
                    &mut consumed,
                    flag,
                    &file.path,
                    format!("[{i}].unlock_flag"),
                );
            }
        }
    }
    if let Some(file) = &catalogs.quests {
        for (i, q) in file.value.entries().iter().enumerate() {
            add(
                &mut consumed,
                &q.started_flag,
                &file.path,
                format!("[{i}].started_flag"),
            );
            add(
                &mut consumed,
                &q.completed_flag,
                &file.path,
                format!("[{i}].completed_flag"),
            );
        }
    }
    (defined, consumed, engine_managed)
}

fn add_conditions(target: &mut FlagLocations, c: &FlagConditions, path: &str, base: String) {
    for (i, f) in c.requires.iter().enumerate() {
        target
            .entry(f.clone())
            .or_default()
            .push(ScenarioLocation::new(path, format!("{base}.requires[{i}]")));
    }
    for (i, f) in c.excludes.iter().enumerate() {
        target
            .entry(f.clone())
            .or_default()
            .push(ScenarioLocation::new(path, format!("{base}.excludes[{i}]")));
    }
}

fn add_actions(target: &mut FlagLocations, a: &DialogueActions, path: &str, base: &str) {
    if let Some(flags) = &a.set_flag {
        for (i, f) in flags.as_slice().iter().enumerate() {
            target
                .entry(f.clone())
                .or_default()
                .push(ScenarioLocation::new(path, format!("{base}.set_flag[{i}]")));
        }
    }
    // A victory flag is set by the battle rather than by the branch, but it is defined here all
    // the same -- otherwise the only place that names it is whatever gate consumes it, and the
    // flag reads as undefined.
    if let Some(battle) = &a.start_battle {
        for (i, f) in battle.on_victory().iter().enumerate() {
            target
                .entry(f.clone())
                .or_default()
                .push(ScenarioLocation::new(
                    path,
                    format!("{base}.start_battle.on_victory[{i}]"),
                ));
        }
    }
}

impl<'a> Validator<'a> {
    pub(super) fn validate_dialogue(&mut self, index: &ReferenceIndex) {
        let documents = self.catalogs.dialogue.clone();
        for file in documents {
            match &file.value {
                DialogueDocument::Cutscene(document) => self.validate_dialogue_actions(
                    &document.on_complete,
                    &file.path,
                    "on_complete",
                    index,
                    DialogueActionContext::Cutscene,
                ),
                DialogueDocument::Entries(document) => {
                    for (entry_index, entry) in document.entries.iter().enumerate() {
                        self.validate_dialogue_actions(
                            &entry.on_complete,
                            &file.path,
                            &format!("entries[{entry_index}].on_complete"),
                            index,
                            DialogueActionContext::Entries,
                        );
                        for (choice_index, choice) in entry.choices.iter().enumerate() {
                            self.validate_dialogue_actions(
                                &choice.on_select,
                                &file.path,
                                &format!(
                                    "entries[{entry_index}].choices[{choice_index}].on_select"
                                ),
                                index,
                                DialogueActionContext::Entries,
                            );
                        }
                    }
                }
                DialogueDocument::LinePool(_) => {}
            }
        }
    }

    fn validate_dialogue_actions(
        &mut self,
        actions: &DialogueActions,
        path: &str,
        base: &str,
        index: &ReferenceIndex,
        context: DialogueActionContext,
    ) {
        for (item_index, item) in actions.give_items.iter().enumerate() {
            self.checked(
                "item",
                &index.items,
                &item.id,
                path,
                format!("{base}.give_items[{item_index}].id"),
            );
        }
        if let Some(member) = &actions.join_party {
            self.checked(
                "party member",
                &index.party,
                member,
                path,
                format!("{base}.join_party"),
            );
        }
        if let Some(transition) = &actions.transition {
            self.checked(
                "map",
                &index.maps,
                &transition.map,
                path,
                format!("{base}.transition.map"),
            );
        }
        if let Some(battle) = &actions.start_battle {
            if context == DialogueActionContext::Cutscene {
                self.error(
                    "dialogue.start_battle_in_cutscene",
                    path,
                    format!("{base}.start_battle"),
                    "`start_battle` is npc-only; a cutscene has no world to return to".to_owned(),
                );
            } else {
                self.checked(
                    "enemy",
                    &index.enemies,
                    battle.enemy_id(),
                    path,
                    format!("{base}.start_battle"),
                );
            }
        }
        if actions.open_transport.is_some() && context == DialogueActionContext::Cutscene {
            self.error(
                "dialogue.open_transport_in_cutscene",
                path,
                format!("{base}.open_transport"),
                "`open_transport` is npc-only; a cutscene has no World atlas to open",
            );
        }
    }

    pub(super) fn validate_flags(&mut self, _index: &ReferenceIndex) {
        // Implemented after all structural checks by a pure inventory walk, so producers and
        // consumers are order-independent and diagnostics point at the first stable location.
        let (defined, consumed, engine_managed) = collect_flag_edges(&self.catalogs);
        for (flag, locations) in &consumed {
            self.report.checked_references += locations.len();
            if !defined.contains_key(flag) {
                for location in locations {
                    self.report.diagnostics.push(ScenarioDiagnostic {
                        severity: DiagnosticSeverity::Error,
                        code: "flag.undefined",
                        location: location.clone(),
                        message: format!("flag `{flag}` is consumed but never produced"),
                    });
                }
            }
        }
        for (flag, locations) in &defined {
            if !consumed.contains_key(flag) && !engine_managed.contains(flag) {
                self.report.diagnostics.push(ScenarioDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "flag.orphan",
                    location: locations[0].clone(),
                    message: format!("flag `{flag}` is produced but never consumed"),
                });
            }
        }
    }
}
