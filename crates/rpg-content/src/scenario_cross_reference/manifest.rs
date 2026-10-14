//! Manifest id references and duplicate ids across catalogs.

use super::{context::Validator, index::ReferenceIndex};
use crate::{
    scenario_duplicate_id::find_duplicate_ids, scenario_manifest::Manifest,
    scenario_root::SCENARIO_MANIFEST_PATH,
};

impl<'a> Validator<'a> {
    pub(super) fn validate_manifest_ids(&mut self, manifest: &Manifest, index: &ReferenceIndex) {
        self.checked(
            "protagonist party member",
            &index.protagonists,
            &manifest.protagonist.id,
            SCENARIO_MANIFEST_PATH,
            "protagonist.id",
        );
        self.checked(
            "class",
            &index.classes,
            &manifest.protagonist.class,
            SCENARIO_MANIFEST_PATH,
            "protagonist.class",
        );
        self.checked(
            "map",
            &index.maps,
            &manifest.start.map,
            SCENARIO_MANIFEST_PATH,
            "start.map",
        );
        if let Some(party) = self.catalogs.party.clone() {
            if index.protagonists.len() != 1 {
                self.error(
                    "party.protagonist_count",
                    &party.path,
                    "party",
                    format!(
                        "expected exactly one protagonist but found {}",
                        index.protagonists.len()
                    ),
                );
            }
            if let Some(member) = party
                .value
                .party
                .iter()
                .find(|member| member.data().id == manifest.protagonist.id)
                && member.data().class_id != manifest.protagonist.class
            {
                self.error(
                    "reference.disagreement",
                    SCENARIO_MANIFEST_PATH,
                    "protagonist.class",
                    format!(
                        "manifest class `{}` disagrees with party member `{}` class `{}`",
                        manifest.protagonist.class,
                        manifest.protagonist.id,
                        member.data().class_id
                    ),
                );
            }
        }
        let intro = self
            .catalogs
            .dialogue
            .iter()
            .find(|document| document.path == manifest.start.intro_dialogue.as_str());
        self.report.checked_references += 1;
        if intro.is_none() {
            self.error(
                "reference.missing",
                SCENARIO_MANIFEST_PATH,
                "start.intro_dialogue",
                "intro path does not identify a loaded dialogue document",
            );
        }
    }

    pub(super) fn validate_duplicates(&mut self, index: &ReferenceIndex) {
        for duplicate in find_duplicate_ids(index.catalog_ids.clone()) {
            self.error(
                "id.duplicate",
                duplicate.duplicate.path().as_str(),
                duplicate.duplicate.field_path(),
                duplicate.to_string(),
            );
        }
    }
}
