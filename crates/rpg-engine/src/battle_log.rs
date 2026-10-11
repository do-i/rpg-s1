//! The most recent battle's log, kept after the battle ends so the field menu can replay it.

use bevy::prelude::*;

/// One logged result, in the order the battle produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BattleLogLine {
    pub(crate) text: String,
    /// Whether an enemy's turn produced the line; the menu colours the two sides apart.
    pub(crate) enemy: bool,
}

/// Every line the last battle's log showed. Cleared when the next battle starts.
#[derive(Debug, Default, Resource)]
pub(crate) struct LastBattleLog {
    lines: Vec<BattleLogLine>,
}

impl LastBattleLog {
    /// A shared empty log, for readers running without the battle plugin.
    pub(crate) fn empty() -> &'static Self {
        static EMPTY: LastBattleLog = LastBattleLog { lines: Vec::new() };
        &EMPTY
    }

    pub(crate) fn clear(&mut self) {
        self.lines.clear();
    }

    /// Records one result. The battle's "Press Enter." cue is dropped: it was an instruction
    /// for that moment, and reads as noise once the fight is over.
    pub(crate) fn push(&mut self, text: &str, enemy: bool) {
        let text = text.strip_suffix(" Press Enter.").unwrap_or(text);
        self.lines.push(BattleLogLine {
            text: text.to_owned(),
            enemy,
        });
    }

    pub(crate) fn lines(&self) -> &[BattleLogLine] {
        &self.lines
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}
