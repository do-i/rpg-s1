//! Palette and node builders shared by the full-screen menus.
//!
//! The field menu grew these first. The title screen's load picker needs the same frame, rules,
//! and status colors, so they live outside `field_menu` rather than being copied into a second
//! screen that would then drift.

use bevy::{ecs::hierarchy::ChildSpawnerCommands, prelude::*};

pub(crate) fn spawn_status_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    node: Node,
    title: &str,
    font: &Handle<Font>,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn((
            node,
            BackgroundColor(Color::srgba_u8(22, 22, 28, 228)),
            BorderColor::all(status_border()),
        ))
        .with_children(|panel| {
            spawn_status_text(panel, title, font, 14.0, status_gold());
            panel.spawn((
                Node {
                    width: percent(100),
                    height: px(1),
                    margin: UiRect::bottom(px(2)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(126, 98, 55, 150)),
            ));
            content(panel);
        });
}

pub(crate) fn spawn_section_rule(parent: &mut ChildSpawnerCommands<'_>) {
    parent.spawn((
        Node {
            width: percent(100),
            height: px(1),
            margin: UiRect::axes(px(0), px(2)),
            ..default()
        },
        BackgroundColor(Color::srgba_u8(126, 98, 55, 100)),
    ));
}

pub(crate) fn spawn_status_text(
    parent: &mut ChildSpawnerCommands<'_>,
    text: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    color: Color,
) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
    ));
}

/// Draws the ember-and-gold rule every menu header opens with.
pub(crate) fn spawn_header_bars(parent: &mut ChildSpawnerCommands<'_>, height: f32, gap: f32) {
    parent.spawn((
        Node {
            width: px(7),
            height: px(height),
            margin: UiRect::right(px(3)),
            ..default()
        },
        BackgroundColor(status_ember()),
    ));
    parent.spawn((
        Node {
            width: px(2),
            height: px(height),
            margin: UiRect::right(px(gap)),
            ..default()
        },
        BackgroundColor(status_gold()),
    ));
}

pub(crate) fn status_ink() -> Color {
    Color::srgb_u8(242, 236, 211)
}

pub(crate) fn status_muted() -> Color {
    Color::srgb_u8(184, 174, 142)
}

pub(crate) fn status_faint() -> Color {
    Color::srgb_u8(116, 108, 90)
}

pub(crate) fn status_gold() -> Color {
    Color::srgb_u8(231, 184, 86)
}

pub(crate) fn status_ember() -> Color {
    Color::srgb_u8(203, 82, 47)
}

pub(crate) fn status_teal() -> Color {
    Color::srgb_u8(67, 166, 160)
}

pub(crate) fn status_violet() -> Color {
    Color::srgb_u8(126, 101, 204)
}

pub(crate) fn status_border() -> Color {
    Color::srgb_u8(126, 98, 55)
}

pub(crate) fn status_border_active() -> Color {
    Color::srgb_u8(235, 190, 89)
}

/// Category prefixes the scenario uses to group map ids, dropped from displayed place names.
const MAP_ID_CATEGORIES: [&str; 4] = ["town", "port", "zone", "dungeon"];

/// Renders a stored map id as a readable place name.
///
/// Saves record the raw map id (`town_02_millhaven`), which is what the slot lists used to show.
/// Grouping prefixes and ordering numbers are authoring structure, not part of the place's name,
/// so they are dropped and the rest is title-cased.
pub(crate) fn location_display_name(raw: &str) -> String {
    let mut segments = raw.split('_').peekable();
    while segments
        .peek()
        .is_some_and(|segment| MAP_ID_CATEGORIES.contains(segment))
    {
        segments.next();
    }
    let name = segments
        .filter(|segment| !segment.is_empty() && !segment.chars().all(|c| c.is_ascii_digit()))
        .map(title_case_word)
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        raw.to_owned()
    } else {
        name
    }
}

fn title_case_word(word: &str) -> String {
    let mut characters = word.chars();
    match characters.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::location_display_name;

    #[test]
    fn map_ids_lose_their_grouping_prefix_and_ordering_numbers() {
        assert_eq!(location_display_name("town_02_millhaven"), "Millhaven");
        assert_eq!(location_display_name("port_town_harborgate"), "Harborgate");
        assert_eq!(
            location_display_name("town_03_ruinwatch_monastery_vaults"),
            "Ruinwatch Monastery Vaults"
        );
        assert_eq!(
            location_display_name("zone_04_ancient_ruins_02_courtyard"),
            "Ancient Ruins Courtyard"
        );
        assert_eq!(location_display_name("town_01_ardel_inn_01"), "Ardel Inn");
    }

    #[test]
    fn text_that_is_not_a_map_id_survives_unharmed() {
        assert_eq!(location_display_name("Unknown"), "Unknown");
        assert_eq!(location_display_name("sample_dungeon_01"), "Sample Dungeon");
    }

    #[test]
    fn an_id_with_nothing_left_to_show_falls_back_to_itself() {
        assert_eq!(location_display_name("zone_01"), "zone_01");
        assert_eq!(location_display_name(""), "");
    }
}
