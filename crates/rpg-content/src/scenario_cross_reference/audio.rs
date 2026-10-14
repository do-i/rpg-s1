//! BGM and SFX index references and their audio files.

use super::{
    context::Validator,
    index::{ReferenceIndex, ability_element_id},
};
use crate::{
    scenario_audio::{AUDIO_ASSET_ROOT, BGM_INDEX_PATH, SFX_INDEX_PATH},
    scenario_class::AbilityKind,
};

impl<'a> Validator<'a> {
    pub(super) fn validate_audio(&mut self, index: &ReferenceIndex) {
        if let Some(bgm) = self.catalogs.bgm.clone() {
            for (category_index, category) in bgm.value.categories.iter().enumerate() {
                for (entry_index, entry) in category.entries.iter().enumerate() {
                    let asset = format!("{AUDIO_ASSET_ROOT}/{}", entry.path);
                    self.checked_path(
                        &bgm.path,
                        format!("categories[{category_index}].entries[{entry_index}]"),
                        &asset,
                    );
                }
            }
        }
        if let Some(sfx) = self.catalogs.sfx.clone() {
            for (category_index, category) in sfx.value.categories.iter().enumerate() {
                for (entry_index, entry) in category.entries.iter().enumerate() {
                    let asset = format!("{AUDIO_ASSET_ROOT}/{}", entry.path);
                    self.checked_path(
                        &sfx.path,
                        format!("categories[{category_index}].entries[{entry_index}]"),
                        &asset,
                    );
                }
            }
        }
        for key in ["title.default", "battle.normal", "battle.boss"] {
            self.checked(
                "BGM",
                &index.bgm,
                key,
                BGM_INDEX_PATH,
                format!("$runtime.{key}"),
            );
        }
        for key in [
            "confirm",
            "cancel",
            "hover",
            "atk_impact",
            "party_hit",
            "enemy_death",
            "flee",
            "denied",
            "encounter",
            "atk_slash",
            "defend",
            "use_item",
            "heal",
            "revive",
            "debuff",
            "atk_buff",
            "def_buff",
        ] {
            self.checked(
                "SFX",
                &index.sfx,
                key,
                SFX_INDEX_PATH,
                format!("$runtime.{key}"),
            );
        }
        let classes = self.catalogs.classes.clone();
        for file in classes {
            for (ability_index, ability) in file.value.abilities.iter().enumerate() {
                if let AbilityKind::Spell(spell) = &ability.kind {
                    let key = format!("spell_{}", ability_element_id(spell.element));
                    self.checked(
                        "SFX",
                        &index.sfx,
                        &key,
                        &file.path,
                        format!("abilities[{ability_index}].element"),
                    );
                }
            }
        }
    }
}
