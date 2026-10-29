//! The most recent battle's log, kept after the battle ends so the field menu can replay it.
//!
//! Each line carries a [`LogKind`] so both the battle screen and the field menu replay colour it
//! the same way: the point is to spot events by colour without reading word by word.

use bevy::prelude::*;

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
    /// Names of the battle's enemies, so they can be told apart from the party in the text.
    pub(crate) enemy_names: Vec<String>,
}

impl BattleLogLine {
    /// The colour of the line's words.
    pub(crate) fn color(&self) -> Color {
        match self.kind {
            LogKind::Item => Color::srgb_u8(0x01, 0xf9, 0x09),
            LogKind::Reward => Color::srgb_u8(0x74, 0xc5, 0x11),
            LogKind::Ability => Color::srgb_u8(0xa5, 0xab, 0x9b),
            LogKind::HitEnemy => Color::srgb_u8(0x48, 0x7f, 0x06),
            LogKind::HitParty => Color::srgb_u8(0xff, 0xa0, 0x80),
            LogKind::Miss => Color::srgb_u8(0x90, 0x70, 0x09),
            LogKind::Info if self.enemy => Color::srgb_u8(0xa2, 0x2d, 0x06),
            LogKind::Info => Color::srgb_u8(0x1d, 0x77, 0x71),
        }
    }

    /// The colour of the damage number, so the amount stands out from the sentence.
    fn number_color(&self) -> Color {
        match self.kind {
            LogKind::HitParty => Color::srgb_u8(0xca, 0x41, 0x1c),
            _ => Color::srgb_u8(0x89, 0xfb, 0x8d),
        }
    }

    /// The sentence split into runs of one colour: party names, the damage amount, the rest.
    fn segments(&self) -> Vec<(std::ops::Range<usize>, Color)> {
        let base = self.color();
        let mut marks: Vec<(std::ops::Range<usize>, Color)> = party_name_ranges(&self.text);
        marks.extend(enemy_name_ranges(&self.text, &self.enemy_names));
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
        "Aric" => Color::srgb_u8(68, 118, 187),
        "Elise" => Color::srgb_u8(210, 101, 145),
        "Reiya" => Color::srgb_u8(134, 41, 168),
        "Jep" => Color::srgb_u8(46, 161, 23),
        "Kael" => Color::srgb_u8(230, 111, 0),
        _ => return None,
    })
}

/// The colour enemy names are written in.
fn enemy_name_color() -> Color {
    Color::srgb_u8(217, 196, 140)
}

/// Byte ranges of `names` in `text`, matched as whole words, longest name first.
fn enemy_name_ranges(text: &str, names: &[String]) -> Vec<(std::ops::Range<usize>, Color)> {
    let mut names = names
        .iter()
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    let mut out: Vec<(std::ops::Range<usize>, Color)> = Vec::new();
    for name in names {
        for (start, _) in text.match_indices(name.as_str()) {
            let end = start + name.len();
            let word_before = text[..start]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric);
            let word_after = text[end..]
                .chars()
                .next()
                .is_some_and(char::is_alphanumeric);
            if !word_before
                && !word_after
                && out.iter().all(|(r, _)| end <= r.start || start >= r.end)
            {
                out.push((start..end, enemy_name_color()));
            }
        }
    }
    out
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
            enemy_names: vec!["Goblin".to_owned()],
        };
        let runs = line
            .segments()
            .into_iter()
            .map(|(range, _)| &line.text[range])
            .collect::<Vec<_>>();
        assert_eq!(
            runs,
            ["Aric", " attacks ", "Goblin", " for ", "14", " damage."]
        );
    }
}
