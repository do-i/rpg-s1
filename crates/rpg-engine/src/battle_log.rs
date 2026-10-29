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
    /// Victory spoils: EXP, GP, level-ups, and loot.
    Reward,
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
            LogKind::Item | LogKind::Reward => status_gold(),
            LogKind::Ability => status_violet(),
            LogKind::HitEnemy => status_teal(),
            LogKind::HitParty => status_ember(),
            LogKind::Miss => status_muted(),
            LogKind::Info if self.enemy => status_ember(),
            LogKind::Info => status_teal(),
        }
    }

    /// The colour of the damage number, so the amount stands out from the sentence.
    fn number_color(&self) -> Color {
        match self.kind {
            LogKind::HitParty => status_ember(),
            _ => Color::srgb_u8(96, 160, 240),
        }
    }

    /// The sentence split into runs of one colour: party names, the damage amount, the rest.
    fn segments(&self) -> Vec<(std::ops::Range<usize>, Color)> {
        let base = self.color();
        let mut marks: Vec<(std::ops::Range<usize>, Color)> = party_name_ranges(&self.text);
        if let Some(range) = damage_amount(&self.text) {
            marks.push((range, self.number_color()));
        }
        marks.sort_by_key(|(range, _)| range.start);
        let mut out = Vec::new();
        let mut at = 0;
        for (range, color) in marks {
            if range.start < at {
                continue;
            }
            if range.start > at {
                out.push((at..range.start, base));
            }
            at = range.end;
            out.push((range, color));
        }
        if at < self.text.len() {
            out.push((at..self.text.len(), base));
        }
        out
    }
}

/// The colour each party member's name is written in.
pub(crate) fn party_name_color(name: &str) -> Option<Color> {
    Some(match name {
        "Aric" => Color::srgb_u8(92, 146, 224),
        "Elise" => Color::srgb_u8(232, 128, 170),
        "Reiya" => Color::srgb_u8(214, 76, 70),
        "Jep" => Color::srgb_u8(110, 184, 96),
        "Kael" => Color::srgb_u8(176, 132, 224),
        _ => return None,
    })
}

/// Byte ranges of party member names in `text`, matched as whole words.
fn party_name_ranges(text: &str) -> Vec<(std::ops::Range<usize>, Color)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices().chain([(text.len(), ' ')]) {
        match (c.is_alphanumeric(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                start = None;
                if let Some(color) = party_name_color(&text[s..i]) {
                    out.push((s..i, color));
                }
            }
            _ => {}
        }
    }
    out
}

/// Byte range of the damage amount in `text`, the digits right before " damage".
fn damage_amount(text: &str) -> Option<std::ops::Range<usize>> {
    let end = text.find(" damage")?;
    let start = text[..end]
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .len();
    (start < end).then_some(start..end)
}

/// Spawns one log line, with party names and the damage amount in their own colours.
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
    let mut segments = line.segments().into_iter();
    let Some((head, color)) = segments.next() else {
        return;
    };
    let mut text = parent.spawn((
        Text::new(&line.text[head]),
        text_font.clone(),
        TextColor(color),
        layout,
    ));
    text.with_children(|text| {
        for (range, color) in segments {
            text.spawn((
                TextSpan::new(&line.text[range]),
                text_font.clone(),
                TextColor(color),
            ));
        }
    });
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

    #[test]
    fn party_names_and_the_amount_split_the_line_into_colour_runs() {
        let line = BattleLogLine {
            text: "Aric attacks Goblin for 14 damage.".to_owned(),
            enemy: false,
            kind: LogKind::HitEnemy,
        };
        let runs = line
            .segments()
            .into_iter()
            .map(|(range, _)| &line.text[range])
            .collect::<Vec<_>>();
        assert_eq!(runs, ["Aric", " attacks Goblin for ", "14", " damage."]);
    }
}
