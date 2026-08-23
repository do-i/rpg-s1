//! Renderer-independent field dialogue resolution and progression.

use std::{collections::BTreeSet, fmt};

use crate::{
    runtime_flags::RuntimeFlags,
    scenario_dialogue::{DialogueActions, DialogueChoice, EntryDialogue},
};

const TYPEWRITER_CHARS_PER_SECOND: f32 = 60.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DialoguePhase {
    Typing,
    Ready,
    Choosing,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChoiceView {
    source_index: usize,
    text: String,
    enabled: bool,
}

impl ChoiceView {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) const fn enabled(&self) -> bool {
        self.enabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DialogueEvent {
    None,
    Revealed,
    Advanced,
    Blocked,
    Apply(Vec<DialogueActions>),
    Cancelled,
}

#[derive(Clone, Debug)]
pub(crate) struct DialogueSession {
    id: String,
    speaker: Option<String>,
    dialogue: EntryDialogue,
    current: usize,
    line: usize,
    revealed_chars: usize,
    phase: DialoguePhase,
    choices: Vec<ChoiceView>,
    selected_choice: usize,
    completed_entries: BTreeSet<usize>,
}

impl DialogueSession {
    pub(crate) fn message(
        id: impl Into<String>,
        speaker: Option<String>,
        lines: Vec<String>,
    ) -> Self {
        let flags = RuntimeFlags::default();
        Self::resolve(
            id,
            speaker,
            EntryDialogue {
                id: None,
                kind: None,
                entries: vec![crate::scenario_dialogue::DialogueEntry {
                    node: None,
                    condition: Default::default(),
                    lines,
                    choices: Vec::new(),
                    next: None,
                    end: true,
                    on_complete: Default::default(),
                }],
            },
            &flags,
        )
        .expect("one terminal message is a valid dialogue graph")
        .expect("unconditional message must resolve")
    }

    pub(crate) fn resolve(
        id: impl Into<String>,
        speaker: Option<String>,
        dialogue: EntryDialogue,
        flags: &RuntimeFlags,
    ) -> Result<Option<Self>, DialogueSessionError> {
        validate_graph(&dialogue)?;
        let Some(current) = dialogue
            .entries
            .iter()
            .position(|entry| entry.node.is_none() && flags.satisfies(&entry.condition))
        else {
            return Ok(None);
        };
        let mut session = Self {
            id: id.into(),
            speaker,
            dialogue,
            current,
            line: 0,
            revealed_chars: 0,
            phase: DialoguePhase::Typing,
            choices: Vec::new(),
            selected_choice: 0,
            completed_entries: BTreeSet::new(),
        };
        session.prime(flags);
        Ok(Some(session))
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn speaker(&self) -> Option<&str> {
        self.speaker.as_deref()
    }

    pub(crate) const fn phase(&self) -> DialoguePhase {
        self.phase
    }

    pub(crate) fn current_line(&self) -> &str {
        self.dialogue.entries[self.current]
            .lines
            .get(self.line)
            .map(String::as_str)
            .unwrap_or("")
    }

    pub(crate) fn visible_text(&self) -> String {
        self.current_line()
            .chars()
            .take(self.revealed_chars)
            .collect()
    }

    pub(crate) fn choices(&self) -> &[ChoiceView] {
        &self.choices
    }

    pub(crate) const fn selected_choice(&self) -> usize {
        self.selected_choice
    }

    pub(crate) fn tick(&mut self, delta_seconds: f32) {
        if self.phase != DialoguePhase::Typing {
            return;
        }
        let total = self.current_line().chars().count();
        let additional = (TYPEWRITER_CHARS_PER_SECOND * delta_seconds.max(0.0)) as usize;
        self.revealed_chars = self.revealed_chars.saturating_add(additional).min(total);
        if self.revealed_chars == total {
            self.phase = DialoguePhase::Ready;
        }
    }

    pub(crate) fn confirm(&mut self, flags: &RuntimeFlags) -> DialogueEvent {
        match self.phase {
            DialoguePhase::Typing => {
                self.revealed_chars = self.current_line().chars().count();
                self.phase = DialoguePhase::Ready;
                DialogueEvent::Revealed
            }
            DialoguePhase::Ready => {
                if self.line + 1 < self.dialogue.entries[self.current].lines.len() {
                    self.line += 1;
                    self.revealed_chars = 0;
                    self.phase = DialoguePhase::Typing;
                    return DialogueEvent::Advanced;
                }
                if !self.dialogue.entries[self.current].choices.is_empty() {
                    self.open_choices(flags);
                    return DialogueEvent::Advanced;
                }
                self.finish_entry(None, flags)
            }
            DialoguePhase::Choosing => {
                let Some(choice) = self.choices.get(self.selected_choice) else {
                    return DialogueEvent::Blocked;
                };
                if !choice.enabled {
                    return DialogueEvent::Blocked;
                }
                let choice =
                    self.dialogue.entries[self.current].choices[choice.source_index].clone();
                self.finish_entry(Some(choice), flags)
            }
            DialoguePhase::Closed => DialogueEvent::None,
        }
    }

    pub(crate) fn move_choice(&mut self, delta: i32) -> bool {
        if self.phase != DialoguePhase::Choosing || self.choices.is_empty() || delta == 0 {
            return false;
        }
        let len = self.choices.len() as i32;
        self.selected_choice = (self.selected_choice as i32 + delta).rem_euclid(len) as usize;
        true
    }

    pub(crate) fn cancel(&mut self) -> DialogueEvent {
        if self.phase == DialoguePhase::Closed {
            return DialogueEvent::None;
        }
        self.phase = DialoguePhase::Closed;
        DialogueEvent::Cancelled
    }

    fn prime(&mut self, flags: &RuntimeFlags) {
        self.line = 0;
        self.revealed_chars = 0;
        self.choices.clear();
        self.selected_choice = 0;
        if self.dialogue.entries[self.current].lines.is_empty() {
            self.open_choices(flags);
        } else {
            self.phase = DialoguePhase::Typing;
        }
    }

    fn open_choices(&mut self, flags: &RuntimeFlags) {
        self.choices = self.dialogue.entries[self.current]
            .choices
            .iter()
            .enumerate()
            .filter(|(_, choice)| flags.satisfies(&choice.condition))
            .map(|(source_index, choice)| ChoiceView {
                source_index,
                text: choice.text.clone(),
                enabled: flags.satisfies(&choice.enabled),
            })
            .collect();
        self.selected_choice = 0;
        self.phase = DialoguePhase::Choosing;
    }

    fn finish_entry(
        &mut self,
        choice: Option<DialogueChoice>,
        flags: &RuntimeFlags,
    ) -> DialogueEvent {
        let entry = &self.dialogue.entries[self.current];
        let mut actions = Vec::new();
        if self.completed_entries.insert(self.current) {
            actions.push(entry.on_complete.clone());
        }
        if let Some(choice) = choice.as_ref() {
            actions.push(choice.on_select.clone());
        }
        let target = choice
            .as_ref()
            .map(|choice| choice.target.as_str())
            .or(entry.next.as_deref());
        if entry.end || target.is_none() {
            self.phase = DialoguePhase::Closed;
        } else {
            self.current = self
                .dialogue
                .entries
                .iter()
                .position(|entry| entry.node.as_deref() == target)
                .expect("validated graph target must exist");
            self.prime(flags);
        }
        DialogueEvent::Apply(actions)
    }
}

pub(crate) fn apply_flag_actions(actions: &DialogueActions, flags: &mut RuntimeFlags) {
    if let Some(set) = actions.set_flag.as_ref() {
        for flag in set.as_slice() {
            flags.set(flag.clone());
        }
    }
    if let Some(unset) = actions.unset_flag.as_ref() {
        for flag in unset.as_slice() {
            flags.unset(flag);
        }
    }
}

fn validate_graph(dialogue: &EntryDialogue) -> Result<(), DialogueSessionError> {
    let mut nodes = BTreeSet::new();
    for entry in &dialogue.entries {
        if let Some(node) = entry.node.as_ref()
            && !nodes.insert(node.as_str())
        {
            return Err(DialogueSessionError::DuplicateNode(node.clone()));
        }
    }
    for entry in &dialogue.entries {
        for target in entry
            .next
            .iter()
            .map(String::as_str)
            .chain(entry.choices.iter().map(|choice| choice.target.as_str()))
        {
            if !nodes.contains(target) {
                return Err(DialogueSessionError::MissingNode(target.to_owned()));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DialogueSessionError {
    DuplicateNode(String),
    MissingNode(String),
}

impl fmt::Display for DialogueSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode(node) => write!(formatter, "dialogue has duplicate node `{node}`"),
            Self::MissingNode(node) => write!(formatter, "dialogue targets missing node `{node}`"),
        }
    }
}

impl std::error::Error for DialogueSessionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{scenario_dialogue::DialogueDocument, scenario_yaml};

    fn dialogue(yaml: &str) -> EntryDialogue {
        let DialogueDocument::Entries(dialogue) = scenario_yaml::from_str(yaml).unwrap() else {
            panic!("expected entry dialogue");
        };
        dialogue
    }

    fn complete_linear(
        session: &mut DialogueSession,
        flags: &RuntimeFlags,
    ) -> Vec<DialogueActions> {
        for _ in 0..32 {
            if let DialogueEvent::Apply(actions) = session.confirm(flags) {
                assert_eq!(session.phase(), DialoguePhase::Closed);
                return actions;
            }
        }
        panic!("linear dialogue did not reach its terminal");
    }

    #[test]
    fn confirm_completes_typewriter_before_advancing_linear_lines() {
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve(
            "linear",
            Some("Maeve".into()),
            dialogue("id: linear\ntype: npc\nentries:\n  - lines: [First, Second]\n"),
            &flags,
        )
        .unwrap()
        .unwrap();
        session.tick(0.02);
        assert_eq!(session.visible_text(), "F");
        assert_eq!(session.confirm(&flags), DialogueEvent::Revealed);
        assert_eq!(session.visible_text(), "First");
        assert_eq!(session.confirm(&flags), DialogueEvent::Advanced);
        assert_eq!(session.current_line(), "Second");
        assert_eq!(session.confirm(&flags), DialogueEvent::Revealed);
        assert!(matches!(session.confirm(&flags), DialogueEvent::Apply(_)));
        assert_eq!(session.phase(), DialoguePhase::Closed);
    }

    #[test]
    fn guide_ardel_traverses_every_story_branch_to_a_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/guide_ardel.yaml"
        ));
        let cases = [
            (
                vec!["story_quest_started", "transport_sail_unlocked"],
                "You've got your sea legs",
            ),
            (
                vec!["story_quest_started", "story_act2_started"],
                "If you're heading east",
            ),
            (
                vec!["story_quest_started", "boss_zone01_defeated"],
                "The forest is quiet again",
            ),
            (vec!["story_quest_started"], "New to Ardel?"),
        ];
        for (flags, expected_start) in cases {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session = DialogueSession::resolve(
                "guide_ardel",
                Some("Ellen".to_owned()),
                dialogue.clone(),
                &flags,
            )
            .unwrap()
            .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let _actions = complete_linear(&mut session, &flags);
        }
    }

    #[test]
    fn ardel_smith_traverses_start_relay_reward_and_repeat_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/ardel_smith.yaml"
        ));
        let cases = [
            (vec!["sq_smith_done"], "An edge like that'll last"),
            (vec!["sq_smith_relayed"], "Ha! So the boy DID"),
            (vec!["sq_smith_started"], "Tomas wandered off"),
            (vec![], "Off to the forest"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("ardel_smith", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            match index {
                1 => {
                    assert_eq!(
                        actions[0].set_flag.as_ref().unwrap().as_slice(),
                        ["sq_smith_done"]
                    );
                    assert_eq!(actions[0].give_items.len(), 1);
                    assert_eq!(actions[0].give_items[0].id, "potion");
                    assert_eq!(actions[0].give_items[0].qty.get(), 3);
                }
                3 => assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["sq_smith_started"]
                ),
                _ => assert_eq!(actions[0], DialogueActions::default()),
            }
        }
    }

    #[test]
    fn ardel_apprentice_traverses_before_active_and_relayed_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/ardel_apprentice.yaml"
        ));
        let cases = [
            (vec!["sq_smith_relayed"], "Tell Bram the new grind"),
            (vec!["sq_smith_started"], "The whetstone?"),
            (vec![], "Bram works me"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("ardel_apprentice", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            if index == 1 {
                assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["sq_smith_relayed"]
                );
            } else {
                assert_eq!(actions[0], DialogueActions::default());
            }
        }
    }

    #[test]
    fn ardel_fisherman_traverses_quest_terminals_and_accounts_for_dead_source_entries() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/ardel_fisherman.yaml"
        ));
        let cases = [
            (vec!["sq_stream_done"], "The keeper came down"),
            (vec!["sq_stream_relayed"], "He'll come?"),
            (vec!["sq_stream_started"], "The keeper's up"),
            (vec![], "The fish swim belly-up some mornings now"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("ardel_fisherman", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            match index {
                1 => {
                    assert_eq!(
                        actions[0].set_flag.as_ref().unwrap().as_slice(),
                        ["sq_stream_done"]
                    );
                    assert_eq!(actions[0].give_items.len(), 1);
                    assert_eq!(actions[0].give_items[0].id, "lure_charm");
                    assert_eq!(actions[0].give_items[0].qty.get(), 2);
                }
                3 => assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["sq_stream_started"]
                ),
                _ => assert_eq!(actions[0], DialogueActions::default()),
            }
        }

        let relevant_flags = [
            "sq_stream_started",
            "sq_stream_relayed",
            "sq_stream_done",
            "boss_zone01_defeated",
            "story_quest_started",
        ];
        for mask in 0..(1 << relevant_flags.len()) {
            let flags = RuntimeFlags::from_bootstrap(
                relevant_flags
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| mask & (1 << index) != 0)
                    .map(|(_, flag)| *flag),
            );
            let session =
                DialogueSession::resolve("ardel_fisherman", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(
                session.current < 4,
                "pinned first-match ordering unexpectedly made a dead entry reachable at mask {mask}"
            );
        }
        assert_eq!(dialogue.entries.len(), 6);
        assert!(dialogue.entries[4].lines[0].starts_with("Fish came back to the stream"));
        assert!(dialogue.entries[5].lines[0].starts_with("The fish swim belly-up"));
    }

    #[test]
    fn ardel_child_traverses_default_and_elise_joined_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/ardel_child.yaml"
        ));
        let cases = [
            (vec![], "Mama says don't go past the well"),
            (vec!["npc_elise_joined"], "Elise went with YOU?"),
        ];
        for (flags, expected_start) in cases {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("ardel_child", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            assert_eq!(
                complete_linear(&mut session, &flags),
                [DialogueActions::default()]
            );
        }
    }

    #[test]
    fn ardel_magic_core_intro_opens_only_after_the_story_starts() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/mc_shop_intro.yaml"
        ));
        assert!(
            DialogueSession::resolve(
                "mc_shop_intro",
                None,
                dialogue.clone(),
                &RuntimeFlags::default(),
            )
            .unwrap()
            .is_none()
        );
        let flags = RuntimeFlags::from_bootstrap(["story_quest_started"]);
        let mut session = DialogueSession::resolve("mc_shop_intro", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(session.current_line().starts_with("Magic Cores?"));
        let actions = complete_linear(&mut session, &flags);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].open_shop,
            Some(crate::scenario_dialogue::DialogueShopKind::MagicCore)
        );
    }

    #[test]
    fn ardel_item_shop_dialogue_reaches_its_service_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/item_shop_ardel.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve("item_shop_ardel", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(session.current_line().starts_with("Welcome!"));
        let actions = complete_linear(&mut session, &flags);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].open_shop,
            Some(crate::scenario_dialogue::DialogueShopKind::Item)
        );
    }

    #[test]
    fn ardel_apothecary_traverses_locked_and_available_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/apothecary_ardel.yaml"
        ));
        let cases = [
            (vec![], "My workshop isn't quite ready", false),
            (vec!["story_quest_started"], "Welcome to my workshop", true),
        ];
        for (flags, expected_start, opens_apothecary) in cases {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("apothecary_ardel", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            assert_eq!(actions[0].open_apothecary.is_some(), opens_apothecary);
        }
    }

    #[test]
    fn ardel_weapon_shop_dialogue_reaches_its_service_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/weapon_shop_ardel.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve("weapon_shop_ardel", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(session.current_line().starts_with("Blades, sticks"));
        let actions = complete_linear(&mut session, &flags);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].open_shop,
            Some(crate::scenario_dialogue::DialogueShopKind::Weapon)
        );
    }

    #[test]
    fn ardel_armor_shop_dialogue_reaches_its_service_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/armor_shop_ardel.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve("armor_shop_ardel", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(session.current_line().starts_with("Cloth and leather"));
        let actions = complete_linear(&mut session, &flags);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].open_shop,
            Some(crate::scenario_dialogue::DialogueShopKind::Armor)
        );
    }

    #[test]
    fn ardel_inn_dialogue_reaches_its_service_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/inn_ardel.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve("inn_ardel", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(
            session
                .current_line()
                .starts_with("Welcome to the Ardel Inn")
        );
        let actions = complete_linear(&mut session, &flags);
        assert_eq!(actions.len(), 1);
        assert!(actions[0].open_inn.is_some());
    }

    #[test]
    fn ardel_shrine_keeper_traverses_quest_and_story_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/ardel_shrine_keeper.yaml"
        ));
        let cases = [
            (vec!["boss_zone10_defeated"], "Warm, isn't it?"),
            (vec!["sq_stream_started"], "Marn sent you?"),
            (vec!["sq_stream_relayed"], "I'll be at the stream"),
            (vec!["story_act2_started"], "So the elder finally told you"),
            (vec![], "Mind the beams"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("ardel_shrine_keeper", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            if index == 1 {
                assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["sq_stream_relayed"]
                );
            } else {
                assert_eq!(actions[0], DialogueActions::default());
            }
        }
    }

    #[test]
    fn bridge_guard_zone5_traverses_only_the_authored_act_two_gate() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/bridge_guard_zone5.yaml"
        ));
        for flags in [vec![], vec!["story_act2_started", "boss_zone04_defeated"]] {
            let flags = RuntimeFlags::from_bootstrap(flags);
            assert!(
                DialogueSession::resolve("bridge_guard_zone5", None, dialogue.clone(), &flags)
                    .unwrap()
                    .is_none()
            );
        }
        let flags = RuntimeFlags::from_bootstrap(["story_act2_started"]);
        let mut session = DialogueSession::resolve("bridge_guard_zone5", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(session.current_line().starts_with("The ruins to the south"));
        assert_eq!(
            complete_linear(&mut session, &flags),
            [DialogueActions::default()]
        );
    }

    #[test]
    fn stronghold_guard_traverses_only_the_authored_act_four_gate() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/stronghold_gate_guard.yaml"
        ));
        for flags in [vec![], vec!["story_act4_started", "boss_zone09_defeated"]] {
            let flags = RuntimeFlags::from_bootstrap(flags);
            assert!(
                DialogueSession::resolve("stronghold_gate_guard", None, dialogue.clone(), &flags,)
                    .unwrap()
                    .is_none()
            );
        }
        let flags = RuntimeFlags::from_bootstrap(["story_act4_started"]);
        let mut session = DialogueSession::resolve("stronghold_gate_guard", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(session.current_line().starts_with("The road ahead"));
        assert_eq!(
            complete_linear(&mut session, &flags),
            [DialogueActions::default()]
        );
    }

    #[test]
    fn ardel_notice_board_traverses_its_authored_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/sign_town_01_ardel.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve("sign_town_01_ardel", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert_eq!(session.current_line(), "Notice Board — Ardel Village");
        assert_eq!(
            complete_linear(&mut session, &flags),
            [DialogueActions::default()]
        );
    }

    #[test]
    fn starting_forest_trail_marker_traverses_its_authored_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_01_starting_forest.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session =
            DialogueSession::resolve("sign_zone_01_starting_forest", None, dialogue, &flags)
                .unwrap()
                .unwrap();
        assert_eq!(session.current_line(), "Trail Marker — Starting Forest");
        assert_eq!(
            complete_linear(&mut session, &flags),
            [DialogueActions::default()]
        );
    }

    #[test]
    fn elise_join_traverses_before_join_offer_and_post_join_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/elise_join.yaml"
        ));
        let cases = [
            (
                vec!["npc_elise_joined", "boss_zone01_defeated"],
                "The forest changed",
            ),
            (vec!["npc_elise_joined"], "I packed bandages"),
            (vec!["story_quest_started"], "Aric. I saw the flame"),
            (vec![], "Morning, Aric"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("elise_join", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            if index == 2 {
                assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["npc_elise_joined"]
                );
                assert_eq!(actions[0].join_party.as_deref(), Some("elise"));
            } else {
                assert_eq!(actions[0], DialogueActions::default());
            }
        }
    }

    #[test]
    fn reiya_join_traverses_pre_offer_offer_and_post_join_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/reiya_join.yaml"
        ));
        assert!(
            DialogueSession::resolve(
                "reiya_join",
                None,
                dialogue.clone(),
                &RuntimeFlags::default()
            )
            .unwrap()
            .is_none()
        );
        let cases = [
            (
                vec!["npc_reiya_joined", "boss_zone02_defeated"],
                "The plains are quiet because the false signal is gone",
            ),
            (
                vec!["npc_reiya_joined"],
                "Millhaven's mills are coughing rust",
            ),
            (
                vec!["story_act2_started"],
                "You described a flame with no heat",
            ),
            (vec!["story_quest_started"], "If you are looking for charms"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("reiya_join", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            if index == 2 {
                assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["npc_reiya_joined"]
                );
                assert_eq!(actions[0].join_party.as_deref(), Some("reiya"));
            } else {
                assert_eq!(actions[0], DialogueActions::default());
            }
        }
    }

    #[test]
    fn millhaven_baker_traverses_start_relay_reward_and_repeat_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_baker.yaml"
        ));
        let cases = [
            (vec!["sq_flour_done"], "Half-batches it is"),
            (vec!["sq_flour_relayed"], "Old grain, he says?"),
            (vec!["sq_flour_started"], "The granary's by the east"),
            (vec![], "Twenty years I've baked here"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("millhaven_baker", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            match index {
                1 => {
                    assert_eq!(
                        actions[0].set_flag.as_ref().unwrap().as_slice(),
                        ["sq_flour_done"]
                    );
                    assert_eq!(actions[0].give_items.len(), 1);
                    assert_eq!(actions[0].give_items[0].id, "hi_potion");
                    assert_eq!(actions[0].give_items[0].qty.get(), 1);
                }
                3 => assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["sq_flour_started"]
                ),
                _ => assert_eq!(actions[0], DialogueActions::default()),
            }
        }
    }

    #[test]
    fn millhaven_granary_traverses_before_relay_and_after_relay_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_granary.yaml"
        ));
        let cases = [
            (vec!["sq_flour_relayed"], "Tell Senna what you like"),
            (vec!["sq_flour_started"], "Senna sent you?"),
            (vec![], "Sacks in, sacks out"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("millhaven_granary", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            if index == 1 {
                assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["sq_flour_relayed"]
                );
            } else {
                assert_eq!(actions[0], DialogueActions::default());
            }
        }
    }

    #[test]
    fn millhaven_miller_traverses_act_three_act_two_and_default_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_miller.yaml"
        ));
        let cases = [
            (vec!["story_act3_started"], "Hear that creak?"),
            (
                vec!["story_act2_started"],
                "Don't go poking around the lower wheel",
            ),
            (vec![], "The mill grinds slower every season"),
        ];
        for (flags, expected_start) in cases {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("millhaven_miller", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            assert_eq!(
                complete_linear(&mut session, &flags),
                [DialogueActions::default()]
            );
        }
    }

    #[test]
    fn millhaven_elder_hint_traverses_every_story_branch_to_a_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_elder_hint.yaml"
        ));
        assert!(
            DialogueSession::resolve(
                "millhaven_elder_hint",
                None,
                dialogue.clone(),
                &RuntimeFlags::default(),
            )
            .unwrap()
            .is_none()
        );
        let cases = [
            (vec!["story_act3_started"], "The mills are slowing"),
            (
                vec!["story_act2_started", "boss_zone02_defeated"],
                "The plains have quieted",
            ),
            (vec!["story_act2_started"], "Millhaven used to trade"),
            (vec!["story_quest_started"], "You have the look of someone"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("millhaven_elder_hint", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            if index == 1 {
                assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["story_act3_started"]
                );
            } else {
                assert_eq!(actions[0], DialogueActions::default());
            }
        }
    }

    #[test]
    fn millhaven_gossip_traverses_reiya_joined_and_default_terminals() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_gossip.yaml"
        ));
        let cases = [
            (
                vec!["npc_reiya_joined"],
                "So Reiya ran off with adventurers",
            ),
            (vec![], "The barons bought the old shrine land"),
        ];
        for (flags, expected_start) in cases {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("millhaven_gossip", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            assert_eq!(
                complete_linear(&mut session, &flags),
                [DialogueActions::default()]
            );
        }
    }

    #[test]
    fn millhaven_carter_traverses_quest_terminals_and_accounts_for_dead_source_entries() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_carter.yaml"
        ));
        let cases = [
            (
                vec!["sq_millstone_done"],
                "The mason's booked for the first thaw",
            ),
            (vec!["sq_millstone_relayed"], "He said yes? To Millhaven?"),
            (
                vec!["sq_millstone_started"],
                "The mason works the yard in Ruinwatch",
            ),
            (vec![], "Hear that grinding?"),
        ];
        for (index, (flags, expected_start)) in cases.into_iter().enumerate() {
            let flags = RuntimeFlags::from_bootstrap(flags);
            let mut session =
                DialogueSession::resolve("millhaven_carter", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(session.current_line().starts_with(expected_start));
            let actions = complete_linear(&mut session, &flags);
            assert_eq!(actions.len(), 1);
            match index {
                1 => {
                    assert_eq!(
                        actions[0].set_flag.as_ref().unwrap().as_slice(),
                        ["sq_millstone_done"]
                    );
                    assert_eq!(actions[0].give_items.len(), 1);
                    assert_eq!(actions[0].give_items[0].id, "tent");
                    assert_eq!(actions[0].give_items[0].qty.get(), 1);
                }
                3 => assert_eq!(
                    actions[0].set_flag.as_ref().unwrap().as_slice(),
                    ["sq_millstone_started"]
                ),
                _ => assert_eq!(actions[0], DialogueActions::default()),
            }
        }

        // Entries [4] (`story_act3_started`) and [5] (unconditional fallback) are dead: the four
        // preceding `sq_millstone_started`/`_relayed`/`_done` entries already exhaustively
        // partition every state via first-match, exactly like `ardel_fisherman`'s trailing pair
        // (ADR 0007, "Dead trailing dialogue entries in six more dialogues").
        let relevant_flags = [
            "sq_millstone_started",
            "sq_millstone_relayed",
            "sq_millstone_done",
            "story_act3_started",
        ];
        for mask in 0..(1 << relevant_flags.len()) {
            let flags = RuntimeFlags::from_bootstrap(
                relevant_flags
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| mask & (1 << index) != 0)
                    .map(|(_, flag)| *flag),
            );
            let session =
                DialogueSession::resolve("millhaven_carter", None, dialogue.clone(), &flags)
                    .unwrap()
                    .unwrap();
            assert!(
                session.current < 4,
                "pinned first-match ordering unexpectedly made a dead entry reachable at mask {mask}"
            );
        }
        assert_eq!(dialogue.entries.len(), 6);
        assert!(
            dialogue.entries[4].lines[0]
                .starts_with("Half my routes are cancelled. Nobody wants 'fragment flour'")
        );
        assert!(dialogue.entries[5].lines[0].starts_with("I run flour east to Ruinwatch and back"));
    }

    #[test]
    fn millhaven_item_shop_dialogue_reaches_its_service_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/item_shop_millhaven.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve("item_shop_millhaven", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(
            session
                .current_line()
                .starts_with("Welcome! Mill trade's been slow")
        );
        let actions = complete_linear(&mut session, &flags);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].open_shop,
            Some(crate::scenario_dialogue::DialogueShopKind::Item)
        );
    }

    #[test]
    fn millhaven_weapon_shop_dialogue_reaches_its_service_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/weapon_shop_millhaven.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve("weapon_shop_millhaven", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(
            session
                .current_line()
                .starts_with("Millhaven steel, ground on our own stones")
        );
        let actions = complete_linear(&mut session, &flags);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].open_shop,
            Some(crate::scenario_dialogue::DialogueShopKind::Weapon)
        );
    }

    #[test]
    fn millhaven_armor_shop_dialogue_reaches_its_service_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/armor_shop_millhaven.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve("armor_shop_millhaven", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(
            session
                .current_line()
                .starts_with("Good boiled leather and a shield that holds")
        );
        let actions = complete_linear(&mut session, &flags);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].open_shop,
            Some(crate::scenario_dialogue::DialogueShopKind::Armor)
        );
    }

    #[test]
    fn millhaven_inn_dialogue_reaches_its_service_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/inn_millhaven.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session = DialogueSession::resolve("inn_millhaven", None, dialogue, &flags)
            .unwrap()
            .unwrap();
        assert!(
            session
                .current_line()
                .starts_with("Welcome to the Millhaven Inn")
        );
        let actions = complete_linear(&mut session, &flags);
        assert_eq!(actions.len(), 1);
        assert!(actions[0].open_inn.is_some());
    }

    #[test]
    fn millhaven_notice_board_traverses_its_authored_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/sign_town_02_millhaven.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session =
            DialogueSession::resolve("sign_town_02_millhaven", None, dialogue, &flags)
                .unwrap()
                .unwrap();
        assert_eq!(session.current_line(), "Notice Board — Millhaven");
        assert_eq!(
            complete_linear(&mut session, &flags),
            [DialogueActions::default()]
        );
    }

    #[test]
    fn open_plains_trail_marker_traverses_its_authored_terminal() {
        let dialogue = dialogue(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_02_open_plains.yaml"
        ));
        let flags = RuntimeFlags::default();
        let mut session =
            DialogueSession::resolve("sign_zone_02_open_plains", None, dialogue, &flags)
                .unwrap()
                .unwrap();
        assert_eq!(session.current_line(), "Trail Marker — Open Plains");
        assert_eq!(
            complete_linear(&mut session, &flags),
            [DialogueActions::default()]
        );
    }

    #[test]
    fn choices_hide_conditions_retain_disabled_rows_and_jump_to_terminal_node() {
        let flags = RuntimeFlags::from_bootstrap(["show_open", "blocked"]);
        let graph = dialogue(
            r#"id: graph
type: npc
entries:
  - lines: [Choose]
    choices:
      - { text: Hidden, target: end, condition: { requires: [hidden] } }
      - { text: Disabled, target: end, enabled: { excludes: [blocked] } }
      - { text: Open, target: end, condition: { requires: [show_open] }, on_select: { set_flag: chose_open } }
  - node: end
    lines: [Done]
    end: true
"#,
        );
        let mut session = DialogueSession::resolve("graph", None, graph, &flags)
            .unwrap()
            .unwrap();
        session.confirm(&flags);
        session.confirm(&flags);
        assert_eq!(session.phase(), DialoguePhase::Choosing);
        assert_eq!(session.choices().len(), 2);
        assert_eq!(session.choices()[0].text(), "Disabled");
        assert!(!session.choices()[0].enabled());
        assert_eq!(session.confirm(&flags), DialogueEvent::Blocked);
        assert!(session.move_choice(1));
        let DialogueEvent::Apply(actions) = session.confirm(&flags) else {
            panic!("enabled choice should apply and jump");
        };
        assert_eq!(actions.len(), 2);
        assert_eq!(session.current_line(), "Done");
        session.confirm(&flags);
        assert!(matches!(session.confirm(&flags), DialogueEvent::Apply(_)));
        assert_eq!(session.phase(), DialoguePhase::Closed);
    }

    #[test]
    fn flag_set_and_unset_effects_are_idempotent() {
        let actions: DialogueActions =
            scenario_yaml::from_str("set_flag: [kept, new]\nunset_flag: [old, absent]\n").unwrap();
        let mut flags = RuntimeFlags::from_bootstrap(["kept", "old"]);
        apply_flag_actions(&actions, &mut flags);
        apply_flag_actions(&actions, &mut flags);
        assert_eq!(flags.iter().collect::<Vec<_>>(), ["kept", "new"]);
    }

    #[test]
    fn invalid_duplicate_and_missing_graph_targets_fail_before_session_start() {
        let flags = RuntimeFlags::default();
        let duplicate = dialogue(
            "entries:\n  - { lines: [Start], next: same }\n  - { node: same, lines: [A] }\n  - { node: same, lines: [B] }\n",
        );
        assert_eq!(
            DialogueSession::resolve("bad", None, duplicate, &flags).unwrap_err(),
            DialogueSessionError::DuplicateNode("same".into())
        );
        let missing = dialogue("entries:\n  - { lines: [Start], next: nowhere }\n");
        assert_eq!(
            DialogueSession::resolve("bad", None, missing, &flags).unwrap_err(),
            DialogueSessionError::MissingNode("nowhere".into())
        );
    }
}
