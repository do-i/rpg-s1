//! The most recent battle's log, kept after the battle ends so the field menu can replay it.
//!
//! Each line carries a [`LogKind`] so both the battle screen and the field menu replay colour it
//! the same way: the point is to spot events by colour without reading word by word.

use bevy::prelude::*;

use crate::menu_chrome::{status_ember, status_gold, status_muted, status_teal, status_violet};

/// What a logged result was, which decides its colour.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum LogKind {
    /// Anything else: coloured by whose turn produced it.
    #[default]
    Info,
    Item,
    Ability,
    /// The party hurt an enemy.
    HitEnemy,
    /// An enemy hurt the party.
    HitParty,
    Miss,
}

/// One logged result, in the order the battle produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BattleLogLine {
    pub(crate) text: String,
    /// Whether an enemy's turn produced the line.
    pub(crate) enemy: bool,
    pub(crate) kind: LogKind,
}

impl BattleLogLine {
    /// The colour of the line's words.
    pub(crate) fn color(&self) -> Color {
        match self.kind {
            LogKind::Item => status_gold(),
            LogKind::Ability => status_violet(),
            LogKind::HitEnemy => status_teal(),
            LogKind::HitParty => status_ember(),
            LogKind::Miss => status_muted(),
            LogKind::Info if self.enemy => status_ember(),
            LogKind::Info => status_teal(),
        }
    }

    /// The lighter colour of the damage number, so the amount stands out from the sentence.
    fn number_color(&self) -> Color {
        match self.kind {
            LogKind::HitParty => Color::srgb_u8(255, 160, 128),
            _ => Color::srgb_u8(160, 232, 224),
        }
    }
}

/// Byte range of the damage amount in `text`, the digits right before " damage".
fn damage_amount(text: &str) -> Option<std::ops::Range<usize>> {
    let end = text.find(" damage")?;
    let start = text[..end]
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .len();
    (start < end).then_some(start..end)
}

/// Spawns one log line, with its damage amount in a lighter shade.
pub(crate) fn spawn_log_line(
    parent: &mut ChildSpawnerCommands<'_>,
    line: &BattleLogLine,
    font: &Handle<Font>,
    size: f32,
    layout: TextLayout,
) {
    let text_font = TextFont {
        font: font.clone().into(),
        font_size: FontSize::Px(size),
        ..default()
    };
    let color = line.color();
    let amount = damage_amount(&line.text);
    let head = amount
        .as_ref()
        .map_or(line.text.as_str(), |r| &line.text[..r.start]);
    let mut text = parent.spawn((Text::new(head), text_font.clone(), TextColor(color), layout));
    if let Some(range) = amount {
        text.with_children(|text| {
            text.spawn((
                TextSpan::new(&line.text[range.clone()]),
                text_font.clone(),
                TextColor(line.number_color()),
            ));
            text.spawn((
                TextSpan::new(&line.text[range.end..]),
                text_font,
                TextColor(color),
            ));
        });
    }
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
    pub(crate) fn push(&mut self, line: BattleLogLine) {
        let text = line
            .text
            .strip_suffix(" Press Enter.")
            .unwrap_or(&line.text);
        self.lines.push(BattleLogLine {
            text: text.to_owned(),
            ..line
        });
    }

    pub(crate) fn lines(&self) -> &[BattleLogLine] {
        &self.lines
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_damage_amount_is_the_number_before_damage() {
        let text = "Elise attacks Goblin 2 for 14 damage. KO!";
        let range = damage_amount(text).expect("a damage line");
        assert_eq!(&text[range], "14");
        assert!(damage_amount("Elise uses Potion!").is_none());
        assert!(damage_amount("Goblin attacks Elise, but misses.").is_none());
    }
}
