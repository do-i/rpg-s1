//! Renderer-independent field dialogue resolution and progression.

use std::{collections::BTreeSet, fmt};

use crate::{
    engine_config::TextSpeed,
    runtime_flags::RuntimeFlags,
    scenario_dialogue::{DialogueActions, DialogueChoice, EntryDialogue},
};

#[cfg(test)]
use crate::scenario_dialogue::DialogueTransportMode;

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
    /// Lines already shown by earlier entries of the current `next:` run, so the page counter
    /// numbers the conversation the player is reading rather than restarting at each hand-off.
    pages_before_current: usize,
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
            pages_before_current: 0,
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

    /// Whether the next confirm closes this dialogue run and applies its completion actions.
    ///
    /// The final line must not carry the same "continue" hint as an intermediate page, or
    /// reward dialogue looks complete before the confirm that actually commits its flags and
    /// gifts. The final acknowledgement stays explicit, and the renderer names it as
    /// a finish action.
    pub(crate) fn finishes_on_confirm(&self) -> bool {
        if self.phase != DialoguePhase::Ready {
            return false;
        }
        let entry = &self.dialogue.entries[self.current];
        self.line + 1 == entry.lines.len()
            && entry.choices.is_empty()
            && (entry.end || entry.next.is_none())
    }

    /// One-based position of the line on screen, and how many the run holds.
    ///
    /// A *run* is everything the player reads before they are asked to do anything: the current
    /// entry plus every entry it hands off to with `next:`. `next` targets a node by name and
    /// resolves without consulting flags, so the chain is walkable in advance and counts as one
    /// conversation, not a restart for each speaker.
    ///
    /// The run stops at a branch: an entry carrying `choices`, or one that ends. The player picks
    /// the arm, and arms differ in length, so there is no further total to promise. The count
    /// starts again once they choose.
    ///
    /// `None` once the session is closed, and for a single-page run, where `1/1` is noise.
    pub(crate) fn page(&self) -> Option<(usize, usize)> {
        if self.phase == DialoguePhase::Closed {
            return None;
        }
        let total = self.pages_before_current + self.pages_remaining_in_run();
        (total > 1).then_some((self.pages_before_current + self.line + 1, total))
    }

    /// Lines from the current entry to the end of its `next:` run.
    ///
    /// The visited set is a cycle guard: `validate_graph` rejects a target that does not exist,
    /// but two entries may legally point at each other, and a counter is no reason to hang.
    fn pages_remaining_in_run(&self) -> usize {
        let mut index = self.current;
        let mut seen = BTreeSet::new();
        let mut total = 0;
        while seen.insert(index) {
            let entry = &self.dialogue.entries[index];
            total += entry.lines.len();
            if entry.end || !entry.choices.is_empty() {
                break;
            }
            let Some(target) = entry.next.as_deref() else {
                break;
            };
            let Some(next) = self
                .dialogue
                .entries
                .iter()
                .position(|entry| entry.node.as_deref() == Some(target))
            else {
                break;
            };
            index = next;
        }
        total
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

    /// Advances the typewriter at the configured reveal rate.
    ///
    /// `speed` comes from `dialogue.text_speed` in `assets/settings.yaml`. `very_fast` is a rate of
    /// zero meaning *instant*, not stalled, so it completes the line in one tick.
    pub(crate) fn tick(&mut self, delta_seconds: f32, speed: TextSpeed) {
        if self.phase != DialoguePhase::Typing {
            return;
        }
        let total = self.current_line().chars().count();
        self.revealed_chars = match speed.chars_per_second() {
            Some(rate) => {
                let additional = (rate * delta_seconds.max(0.0)) as usize;
                self.revealed_chars.saturating_add(additional).min(total)
            }
            None => total,
        };
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
            // A `next:` hand-off continues the same run of pages; a choice starts a new one,
            // because the arms differ in length and nothing could have counted them in advance.
            self.pages_before_current = if choice.is_some() {
                0
            } else {
                self.pages_before_current + entry.lines.len()
            };
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

pub(crate) fn validate_graph(dialogue: &EntryDialogue) -> Result<(), DialogueSessionError> {
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
mod tests;
