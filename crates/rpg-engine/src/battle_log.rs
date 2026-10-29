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

    /// The sentence split into runs of one colour: names, damage amounts, the rest.
    ///
    /// An amount is blue-green when the last name before it is an enemy (the party hurt it)
    /// and red when it is a party member (the party was hurt).
    fn segments(&self) -> Vec<(std::ops::Range<usize>, Color)> {
        let base = self.color();
        let party = party_name_ranges(&self.text);
        let enemies = enemy_name_ranges(&self.text, &self.enemy_names);
        let mut marks = party.clone();
        marks.extend(enemies.iter().cloned());
        for range in damage_amounts(&self.text) {
            let victim = party
                .iter()
                .map(|(r, _)| (r.start, false))
                .chain(enemies.iter().map(|(r, _)| (r.start, true)))
                .filter(|(start, _)| *start < range.start)
                .max_by_key(|(start, _)| *start);
            let color = match victim {
                Some((_, true)) => Color::srgb_u8(0x89, 0xfb, 0x8d),
                Some((_, false)) => Color::srgb_u8(0xca, 0x41, 0x1c),
                None if self.kind == LogKind::HitParty => Color::srgb_u8(0xca, 0x41, 0x1c),
                None => Color::srgb_u8(0x89, 0xfb, 0x8d),
            };
            marks.push((range, color));
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

/// Byte ranges of every damage amount in `text`, the digits right before " damage".
fn damage_amounts(text: &str) -> Vec<std::ops::Range<usize>> {
    text.match_indices(" damage")
        .filter_map(|(end, _)| {
            let start = text[..end]
                .trim_end_matches(|c: char| c.is_ascii_digit())
                .len();
            (start < end).then_some(start..end)
        })
        .collect()
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
        let ranges = damage_amounts(text);
        assert_eq!(&text[ranges[0].clone()], "14");
        assert!(damage_amounts("Elise uses Potion!").is_empty());
        assert!(damage_amounts("Goblin attacks Elise, but misses.").is_empty());
    }

    #[test]
    fn an_amount_takes_the_colour_of_whoever_it_hurt() {
        let line = BattleLogLine {
            text: "Elise uses Fire! Goblin takes 12 damage. Aric takes 3 damage.".to_owned(),
            enemy: false,
            kind: LogKind::Ability,
            enemy_names: vec!["Goblin".to_owned()],
        };
        let amounts = line
            .segments()
            .into_iter()
            .filter(|(range, _)| line.text[range.clone()].parse::<u32>().is_ok())
            .map(|(_, color)| color)
            .collect::<Vec<_>>();
        assert_eq!(
            amounts,
            [
                Color::srgb_u8(0x89, 0xfb, 0x8d),
                Color::srgb_u8(0xca, 0x41, 0x1c)
            ]
        );
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
