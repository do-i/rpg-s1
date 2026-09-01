//! Source-authored dialogue schemas.
//!
//! The pinned `0897035` scenario contains 91 mapping-root dialogue files. One is a cutscene,
//! 89 contain ordered conditional entries, and one is a reusable line pool. Conditional
//! dialogue branches by selecting the first entry whose [`FlagConditions`] match. The copied
//! source corpus is linear; the optional node, choice, `next`, and terminal fields extend that
//! wire format for the M5 runtime without changing any source-authored document. Speaker names
//! and portraits still come from map NPC metadata.
//!
//! Entry documents may omit `id` (one pinned file does) and `type` (thirteen do). The line-pool
//! utility also has no id. In those cases the containing filename stem is the effective id,
//! matching the direct source-format contract. Missing conditions and completion actions are
//! explicit Python loader defaults. The pinned files contain no YAML nulls, so null is not accepted as shorthand for
//! any of those defaults.

use std::num::NonZeroU32;

use bevy::{asset::Asset, reflect::TypePath};
use serde::{Deserialize, Deserializer, de};

use crate::scenario_condition::FlagConditions;
use crate::scenario_spatial::Position;
use crate::scenario_yaml::{deserialize_string, deserialize_strings};

/// One complete YAML document beneath `data/dialogue/`.
#[derive(Asset, Clone, Debug, Deserialize, Eq, PartialEq, TypePath)]
#[serde(untagged)]
pub enum DialogueDocument {
    Cutscene(CutsceneDialogue),
    Entries(EntryDialogue),
    LinePool(DialogueLinePool),
}

impl DialogueDocument {
    /// Returns the authored id, or the containing filename stem for an id-less utility file.
    pub fn effective_id<'a>(&'a self, filename_stem: &'a str) -> &'a str {
        match self {
            Self::Cutscene(dialogue) => &dialogue.id,
            Self::Entries(dialogue) => dialogue.id.as_deref().unwrap_or(filename_stem),
            Self::LinePool(_) => filename_stem,
        }
    }
}

/// A linear cutscene document such as the new-game introduction.
#[derive(Asset, Clone, Debug, Deserialize, Eq, PartialEq, TypePath)]
#[serde(deny_unknown_fields)]
pub struct CutsceneDialogue {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(rename = "type")]
    pub kind: CutsceneKind,
    #[serde(deserialize_with = "deserialize_strings")]
    pub lines: Vec<String>,
    #[serde(default)]
    pub on_complete: DialogueActions,
}

/// The only cutscene discriminant authored by the pinned scenario.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CutsceneKind {
    Cutscene,
}

/// A dialogue containing ordered, flag-selected entries.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EntryDialogue {
    /// Authored identity. Missing values use the containing filename stem.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub id: Option<String>,
    /// Missing in the pinned inn, item-shop, and port-master documents.
    #[serde(
        default,
        rename = "type",
        deserialize_with = "deserialize_present_option"
    )]
    pub kind: Option<EntryDialogueKind>,
    pub entries: Vec<DialogueEntry>,
}

/// Closed entry-document kinds present in the pinned corpus.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EntryDialogueKind {
    Npc,
    Guide,
    Board,
}

/// One candidate branch. Source order determines first-match precedence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DialogueEntry {
    /// Optional graph-node identity. Node entries are reached by `next` or a choice target.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub node: Option<String>,
    #[serde(default)]
    pub condition: FlagConditions,
    #[serde(deserialize_with = "deserialize_strings")]
    pub lines: Vec<String>,
    #[serde(default)]
    pub choices: Vec<DialogueChoice>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub next: Option<String>,
    #[serde(default)]
    pub end: bool,
    #[serde(default)]
    pub on_complete: DialogueActions,
}

/// One visible graph choice with independent visibility and enabled conditions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DialogueChoice {
    #[serde(deserialize_with = "deserialize_string")]
    pub text: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub target: String,
    #[serde(default)]
    pub condition: FlagConditions,
    #[serde(default)]
    pub enabled: FlagConditions,
    #[serde(default)]
    pub on_select: DialogueActions,
}

/// An id-less reusable list of authored lines.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DialogueLinePool {
    #[serde(deserialize_with = "deserialize_strings")]
    pub lines: Vec<String>,
}

/// Completion effects recognized in the pinned dialogue corpus.
///
/// Missing fields mean no effect. Runtime application, idempotency, and reference validation
/// belong to later milestones; this type only preserves the strict source wire values.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DialogueActions {
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub set_flag: Option<SetFlagAction>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub unset_flag: Option<SetFlagAction>,
    #[serde(default)]
    pub give_items: Vec<DialogueItemGrant>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub join_party: Option<String>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub transition: Option<DialogueTransition>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub open_shop: Option<DialogueShopKind>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub open_inn: Option<ActionTrigger>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub open_apothecary: Option<ActionTrigger>,
    /// A scripted battle against one named enemy, fought as soon as the branch closes.
    ///
    /// The source wire form is a bare enemy id (`start_battle: cinder_marshal`), and the source
    /// documents it as `npc`-only — a cutscene has no world to return to, so
    /// `scenario_cross_reference` rejects it there rather than letting it load and strand the
    /// player. The pinned Python engine parses the key and hands it back to its caller
    /// (`dialogue_engine.py:119`); no caller ever read it, so the runtime behaviour below is this
    /// port's, not a transcription.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub start_battle: Option<String>,
}

/// The source permits one flag string or an ordered list of flag strings.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum SetFlagAction {
    One(OneFlag),
    Many(ManyFlags),
}

impl SetFlagAction {
    pub fn as_slice(&self) -> &[String] {
        match self {
            Self::One(flag) => std::slice::from_ref(&flag.0),
            Self::Many(flags) => &flags.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct OneFlag(#[serde(deserialize_with = "deserialize_string")] pub String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct ManyFlags(#[serde(deserialize_with = "deserialize_strings")] pub Vec<String>);

/// An item quantity granted after a dialogue branch completes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DialogueItemGrant {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    pub qty: NonZeroU32,
}

/// A source-authored map transition from the introductory cutscene.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DialogueTransition {
    #[serde(deserialize_with = "deserialize_string")]
    pub map: String,
    pub position: Position,
    pub fade: DialogueFade,
}

/// The sole dialogue-transition fade value in the pinned corpus.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DialogueFade {
    In,
}

/// The four service catalog kinds selected by dialogue.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DialogueShopKind {
    Item,
    Weapon,
    Armor,
    MagicCore,
}

/// A presence-only action whose pinned source spelling must be `true`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionTrigger;

impl<'de> Deserialize<'de> for ActionTrigger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if bool::deserialize(deserializer)? {
            Ok(Self)
        } else {
            Err(de::Error::custom(
                "expected true for an enabled dialogue action",
            ))
        }
    }
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string(deserializer).map(Some)
}

fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{
        ActionTrigger, DialogueDocument, DialogueFade, DialogueShopKind, EntryDialogueKind,
        SetFlagAction,
    };
    use crate::scenario_spatial::Position;
    use crate::scenario_yaml;

    #[test]
    fn a_branch_can_name_the_enemy_it_starts_a_battle_with() {
        let document: DialogueDocument = scenario_yaml::from_str(
            "id: marshal_duel\n\
             type: npc\n\
             entries:\n\
             \x20 - lines: [\"Then draw.\"]\n\
             \x20   end: true\n\
             \x20   on_complete:\n\
             \x20     set_flag: marshal_duel_begun\n\
             \x20     start_battle: cinder_marshal\n",
        )
        .expect("the source wire form is a bare enemy id");
        let DialogueDocument::Entries(dialogue) = document else {
            panic!("type: npc should select the entry document shape");
        };
        let actions = &dialogue.entries[0].on_complete;
        assert_eq!(actions.start_battle.as_deref(), Some("cinder_marshal"));
        assert_eq!(
            actions.set_flag.as_ref().unwrap().as_slice(),
            ["marshal_duel_begun"]
        );
    }

    #[test]
    fn a_branch_without_the_verb_leaves_it_unset() {
        let document: DialogueDocument = scenario_yaml::from_str(
            "id: plain\n\
             type: npc\n\
             entries:\n\
             \x20 - lines: [\"Fair weather.\"]\n\
             \x20   end: true\n",
        )
        .unwrap();
        let DialogueDocument::Entries(dialogue) = document else {
            panic!("type: npc should select the entry document shape");
        };
        assert_eq!(dialogue.entries[0].on_complete.start_battle, None);
    }

    #[test]
    fn loads_source_shaped_intro_cutscene_without_changing_text_or_order() {
        let document: DialogueDocument = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/dialogue-intro-cutscene.yaml"
        ))
        .expect("the source-shaped intro fixture should deserialize");

        let DialogueDocument::Cutscene(intro) = document else {
            panic!("type: cutscene should select the cutscene document shape");
        };
        assert_eq!(intro.id, "ember_intro");
        assert_eq!(
            intro.lines,
            [
                "Before the bell rang, the valley held its breath.",
                "...",
                "Then the blue ember answered Mira's name — once.",
            ]
        );
        assert_eq!(
            intro.on_complete.set_flag.unwrap().as_slice(),
            ["story_ember_seen"]
        );
        let transition = intro.on_complete.transition.unwrap();
        assert_eq!(transition.map, "town_test_haven");
        assert_eq!(transition.position, Position::new(12, 8));
        assert_eq!(transition.fade, DialogueFade::In);
    }

    #[test]
    fn loads_ordered_branching_npc_entries_and_every_corpus_action_shape() {
        let document: DialogueDocument = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/dialogue-branching-npc.yaml"
        ))
        .expect("the source-shaped branching fixture should deserialize");
        assert_eq!(document.effective_id("ignored_stem"), "mira_join");

        let DialogueDocument::Entries(dialogue) = document else {
            panic!("type: npc should select the entry document shape");
        };
        assert_eq!(dialogue.kind, Some(EntryDialogueKind::Npc));
        assert_eq!(dialogue.entries.len(), 4);

        let joined = &dialogue.entries[0];
        assert_eq!(joined.condition.requires, ["npc_mira_joined"]);
        assert_eq!(
            joined.lines,
            ["The road is quieter with two sets of boots."]
        );
        assert_eq!(
            joined.on_complete.open_shop,
            Some(DialogueShopKind::MagicCore)
        );

        let invitation = &dialogue.entries[1];
        assert_eq!(invitation.condition.requires, ["story_ember_seen"]);
        assert_eq!(invitation.condition.excludes, ["npc_mira_joined"]);
        let SetFlagAction::Many(flags) = invitation.on_complete.set_flag.as_ref().unwrap() else {
            panic!("the list spelling must remain a list variant");
        };
        assert_eq!(flags.0, ["npc_mira_joined", "story_road_open"]);
        assert_eq!(invitation.on_complete.join_party.as_deref(), Some("mira"));
        assert_eq!(invitation.on_complete.give_items[0].id, "field_tonic");
        assert_eq!(invitation.on_complete.give_items[0].qty.get(), 2);

        assert_eq!(
            dialogue.entries[2].on_complete.open_inn,
            Some(ActionTrigger)
        );
        assert_eq!(
            dialogue.entries[3].on_complete.open_apothecary,
            Some(ActionTrigger)
        );
    }

    #[test]
    fn applies_only_observed_missing_field_and_filename_identity_defaults() {
        let document: DialogueDocument =
            scenario_yaml::from_str("entries:\n  - lines: [\"No conditions or effects.\"]\n")
                .expect("id, type, condition, and on_complete are observed optional fields");
        assert_eq!(
            document.effective_id("port_master_intro"),
            "port_master_intro"
        );

        let DialogueDocument::Entries(dialogue) = document else {
            panic!("entries root should select the entry shape");
        };
        assert!(dialogue.id.is_none());
        assert!(dialogue.kind.is_none());
        assert_eq!(dialogue.entries[0].condition, Default::default());
        assert_eq!(dialogue.entries[0].on_complete, Default::default());

        let pool: DialogueDocument =
            scenario_yaml::from_str("lines: [\"First excuse.\", \"Second excuse.\"]\n")
                .expect("the id-less utility line-pool shape should deserialize");
        assert_eq!(pool.effective_id("guide_excuses"), "guide_excuses");
    }

    #[test]
    fn loads_runtime_choice_nodes_disabled_conditions_and_unset_effects() {
        let document: DialogueDocument = scenario_yaml::from_str(
            r#"
id: invented_graph
type: npc
entries:
  - lines: ["Choose."]
    choices:
      - text: "Open road"
        target: road
        condition: { requires: [road_seen] }
        enabled: { excludes: [road_blocked] }
        on_select: { set_flag: choice_taken }
      - text: "Locked road"
        target: road
        enabled: { requires: [never_set] }
  - node: road
    lines: ["The road opens."]
    end: true
    on_complete: { unset_flag: [road_seen, road_blocked] }
"#,
        )
        .unwrap();
        let DialogueDocument::Entries(dialogue) = document else {
            panic!("entry graph should select entry document");
        };
        assert_eq!(dialogue.entries[0].choices.len(), 2);
        assert_eq!(dialogue.entries[0].choices[0].target, "road");
        assert_eq!(dialogue.entries[1].node.as_deref(), Some("road"));
        assert!(dialogue.entries[1].end);
        assert_eq!(
            dialogue.entries[1]
                .on_complete
                .unset_flag
                .as_ref()
                .unwrap()
                .as_slice(),
            ["road_seen", "road_blocked"]
        );
    }

    #[test]
    fn rejects_unknown_graph_fields_nulls_coercions_and_invalid_action_variants() {
        for document in [
            "id: intro\ntype: cutscene\nlines: [hello]\nchoice: continue\n",
            "id: npc\ntype: npc\nentries: [{ lines: [hello], jump: goodbye }]\n",
            "id: npc\ntype: npc\nentries: [{ lines: [hello], speaker: Mira }]\n",
            "id: npc\ntype: npc\nentries: [{ lines: [hello], condition: null }]\n",
            "id: npc\ntype: npc\nentries: [{ lines: [hello], on_complete: null }]\n",
            "id: npc\ntype: npc\nentries: [{ lines: [42] }]\n",
            "id: npc\ntype: npc\nentries: [{ lines: [hello], on_complete: { set_flag: true } }]\n",
            "id: npc\ntype: npc\nentries: [{ lines: [hello], on_complete: { give_items: [{ id: tonic, qty: 0 }] } }]\n",
            "id: npc\ntype: npc\nentries: [{ lines: [hello], on_complete: { open_shop: trinket } }]\n",
            "id: npc\ntype: npc\nentries: [{ lines: [hello], on_complete: { open_inn: false } }]\n",
            "id: npc\ntype: cutscene\nentries: [{ lines: [hello] }]\n",
        ] {
            assert!(
                scenario_yaml::from_str::<DialogueDocument>(document).is_err(),
                "unexpectedly accepted:\n{document}"
            );
        }
    }

    #[test]
    #[ignore = "requires the separately pinned Python scenario checkout"]
    fn audits_every_pinned_dialogue_file_when_source_is_available() {
        let dialogue_dir = std::env::var_os("RPG_S1_PINNED_DIALOGUE_DIR")
            .expect("RPG_S1_PINNED_DIALOGUE_DIR must name the pinned data/dialogue directory");
        let mut files = fs::read_dir(Path::new(&dialogue_dir))
            .expect("dialogue directory should be readable")
            .map(|entry| entry.expect("directory entry should be readable").path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "yaml")
            })
            .collect::<Vec<_>>();
        files.sort();

        let mut cutscenes = 0;
        let mut entry_documents = 0;
        let mut line_pools = 0;
        let mut idless = 0;
        for path in &files {
            let document = fs::read_to_string(path).expect("dialogue YAML should be readable");
            let dialogue: DialogueDocument = scenario_yaml::from_str(&document)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            match dialogue {
                DialogueDocument::Cutscene(_) => cutscenes += 1,
                DialogueDocument::Entries(entries) => {
                    entry_documents += 1;
                    idless += usize::from(entries.id.is_none());
                }
                DialogueDocument::LinePool(_) => {
                    line_pools += 1;
                    idless += 1;
                }
            }
        }

        assert_eq!(files.len(), 91);
        assert_eq!((cutscenes, entry_documents, line_pools), (1, 89, 1));
        assert_eq!(idless, 2);
    }
}
