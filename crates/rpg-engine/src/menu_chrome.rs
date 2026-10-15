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
            spawn_panel_rule(panel);
            content(panel);
        });
}

/// A [`spawn_status_panel`] that marks keyboard focus with a brighter, thicker rim.
///
/// The rim is an [`Outline`], drawn outside the node, so focus never changes the panel's border
/// width and nothing inside it reflows when focus moves between columns. Unfocused panels keep a
/// transparent outline rather than none, so toggling focus does not add or remove components.
pub(crate) fn spawn_focus_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    node: Node,
    title: &str,
    font: &Handle<Font>,
    focused: bool,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn((
            node,
            BackgroundColor(Color::srgba_u8(22, 22, 28, 228)),
            BorderColor::all(if focused {
                status_border_active()
            } else {
                status_border()
            }),
            Outline {
                width: px(2),
                offset: px(0),
                color: if focused {
                    status_border_active()
                } else {
                    Color::NONE
                },
            },
        ))
        .with_children(|panel| {
            spawn_status_text(panel, title, font, 14.0, status_gold());
            spawn_panel_rule(panel);
            content(panel);
        });
}

/// The rule under a status panel's title; also closes a list off above its footer.
pub(crate) fn spawn_panel_rule(parent: &mut ChildSpawnerCommands<'_>) {
    parent.spawn((
        Node {
            width: percent(100),
            height: px(1),
            flex_shrink: 0.0,
            margin: UiRect::bottom(px(2)),
            ..default()
        },
        BackgroundColor(Color::srgba_u8(126, 98, 55, 150)),
    ));
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

/// The green the shop family reserves for an affordable price and a ready recipe.
pub(crate) fn status_ready() -> Color {
    Color::srgb_u8(132, 196, 111)
}

/// Draws the labelled `value / maximum` bar the party panels share.
pub(crate) fn spawn_meter(
    parent: &mut ChildSpawnerCommands<'_>,
    label: &str,
    value: u32,
    maximum: u32,
    font: &Handle<Font>,
    fill: Color,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            ..default()
        })
        .with_children(|meter| {
            meter
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|label_row| {
                    spawn_status_text(label_row, label, font, 13.0, status_muted());
                    spawn_status_text(
                        label_row,
                        format!("{value} / {maximum}"),
                        font,
                        13.0,
                        status_ink(),
                    );
                });
            meter
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(8),
                        border_radius: BorderRadius::all(px(4)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(17, 17, 22)),
                ))
                .with_children(|track| {
                    track.spawn((
                        Node {
                            width: percent(meter_percent(value, maximum)),
                            height: percent(100),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(fill),
                    ));
                });
        });
}

pub(crate) fn meter_percent(value: u32, maximum: u32) -> f32 {
    if maximum == 0 {
        0.0
    } else {
        (value as f32 / maximum as f32 * 100.0).clamp(0.0, 100.0)
    }
}

/// First visible index of a list that scrolls only far enough to keep `selected` on screen.
///
/// This is the source `ScrollListState` rule (`engine/common/scroll_list.py`), which the quest
/// register and the shop family both need: the window holds still until the cursor would leave
/// it, rather than paging in fixed blocks.
pub(crate) fn window_start(selected: usize, len: usize, rows: usize) -> usize {
    if rows == 0 || len <= rows {
        return 0;
    }
    selected.saturating_sub(rows - 1).min(len - rows)
}

/// Which edges of a windowed list's visible card have hidden cards beyond them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ScrollFades {
    pub(crate) top: bool,
    pub(crate) bottom: bool,
}

impl ScrollFades {
    /// The fades for the card at `offset` within a window of `shown` cards starting at `first`
    /// in a list of `len`: the first card fades upward when the window has scrolled, and the
    /// last fades downward when more cards wait below.
    pub(crate) const fn for_row(offset: usize, shown: usize, first: usize, len: usize) -> Self {
        Self {
            top: offset == 0 && first > 0,
            bottom: offset + 1 == shown && first + shown < len,
        }
    }

    const fn any(self) -> bool {
        self.top || self.bottom
    }
}

/// The half-card shade marking a list edge with more cards past it.
#[derive(Clone, Copy, Component, Debug, Eq, PartialEq)]
pub(crate) enum ScrollFade {
    Top,
    Bottom,
}

/// The scroll fade's colour; its alpha ramps from clear at the card's middle to these stops.
const SCROLL_FADE_SHADE: Color = Color::srgb(0.02, 0.02, 0.03);
/// Shade alpha a quarter of the card from its edge, and at the edge itself.
const SCROLL_FADE_MID_ALPHA: f32 = 0.8;
const SCROLL_FADE_EDGE_ALPHA: f32 = 0.96;

/// Background and border of one selectable list card, highlighted while `selected`.
pub(crate) fn selectable_card_colors(selected: bool) -> (BackgroundColor, BorderColor) {
    if selected {
        (
            BackgroundColor(Color::srgba_u8(72, 49, 25, 218)),
            BorderColor::all(status_border_active()),
        )
    } else {
        (
            BackgroundColor(Color::srgba_u8(10, 10, 14, 148)),
            BorderColor::all(Color::srgba_u8(126, 98, 55, 90)),
        )
    }
}

/// The rows a scrolled list shows: each visible row's index into the full list, with the fades its
/// card needs to say more rows are hidden above or below.
pub(crate) fn visible_rows(
    first: usize,
    len: usize,
    rows: usize,
) -> impl Iterator<Item = (usize, ScrollFades)> {
    let shown = len.saturating_sub(first).min(rows);
    (0..shown).map(move |offset| {
        (
            first + offset,
            ScrollFades::for_row(offset, shown, first, len),
        )
    })
}

/// Spawns one card of a windowed list, shading the half of it that faces hidden cards.
///
/// A card with nothing hidden beyond it is spawned exactly as `spawn_card` builds it. An edge
/// card is wrapped in a full-width box so the shade can be laid over it absolutely without
/// knowing the card's height or touching its own children.
pub(crate) fn spawn_windowed_card(
    parent: &mut ChildSpawnerCommands<'_>,
    fades: ScrollFades,
    spawn_card: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    if !fades.any() {
        spawn_card(parent);
        return;
    }
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|wrapper| {
            spawn_card(wrapper);
            spawn_scroll_fades(wrapper, fades);
        });
}

/// Lays the shades for `fades` over the node being built, which must not clip them away.
///
/// Cards that size themselves by flex (and so cannot sit in [`spawn_windowed_card`]'s wrapper)
/// call this as their last child instead.
pub(crate) fn spawn_scroll_fades(parent: &mut ChildSpawnerCommands<'_>, fades: ScrollFades) {
    for (fade, shown) in [
        (ScrollFade::Top, fades.top),
        (ScrollFade::Bottom, fades.bottom),
    ] {
        if shown {
            spawn_scroll_fade(parent, fade);
        }
    }
}

fn spawn_scroll_fade(parent: &mut ChildSpawnerCommands<'_>, fade: ScrollFade) {
    // The shade deepens quickly toward the edge the hidden cards lie past.
    let stops = vec![
        ColorStop::percent(SCROLL_FADE_SHADE.with_alpha(0.0), 0.0),
        ColorStop::percent(SCROLL_FADE_SHADE.with_alpha(SCROLL_FADE_MID_ALPHA), 50.0),
        ColorStop::percent(SCROLL_FADE_SHADE.with_alpha(SCROLL_FADE_EDGE_ALPHA), 100.0),
    ];
    let (gradient, top, bottom, radius) = match fade {
        ScrollFade::Top => (
            LinearGradient::to_top(stops),
            px(0),
            auto(),
            BorderRadius::top(px(4)),
        ),
        ScrollFade::Bottom => (
            LinearGradient::to_bottom(stops),
            auto(),
            px(0),
            BorderRadius::bottom(px(4)),
        ),
    };
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top,
            bottom,
            height: percent(50),
            border_radius: radius,
            ..default()
        },
        BackgroundGradient(vec![gradient.into()]),
        fade,
        Name::new("Scroll fade"),
    ));
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
    use super::{ScrollFades, location_display_name, meter_percent, visible_rows, window_start};

    #[test]
    fn only_edge_cards_facing_hidden_cards_fade() {
        // Everything fits: nothing fades.
        assert_eq!(ScrollFades::for_row(0, 5, 0, 5), ScrollFades::default());
        assert_eq!(ScrollFades::for_row(4, 5, 0, 5), ScrollFades::default());
        // At the top of a longer list only the last card fades, downward.
        assert_eq!(
            ScrollFades::for_row(6, 7, 0, 20),
            ScrollFades {
                top: false,
                bottom: true
            }
        );
        assert_eq!(ScrollFades::for_row(0, 7, 0, 20), ScrollFades::default());
        // Scrolled to the end only the first card fades, upward.
        assert_eq!(
            ScrollFades::for_row(0, 7, 13, 20),
            ScrollFades {
                top: true,
                bottom: false
            }
        );
        assert_eq!(ScrollFades::for_row(6, 7, 13, 20), ScrollFades::default());
        // A one-card window in the middle fades both ways.
        assert_eq!(
            ScrollFades::for_row(0, 1, 3, 9),
            ScrollFades {
                top: true,
                bottom: true
            }
        );
        // Middle cards never fade.
        assert_eq!(ScrollFades::for_row(3, 7, 5, 20), ScrollFades::default());
    }

    #[test]
    fn meter_fill_clamps_to_the_track_and_survives_a_zero_maximum() {
        assert_eq!(meter_percent(3, 0), 0.0);
        assert_eq!(meter_percent(25, 100), 25.0);
        assert_eq!(meter_percent(125, 100), 100.0);
    }

    #[test]
    fn a_list_window_holds_still_until_the_cursor_would_leave_it() {
        // Short lists never scroll, whichever row is selected.
        assert_eq!(window_start(0, 5, 7), 0);
        assert_eq!(window_start(4, 5, 7), 0);
        // Long lists hold at the top until the cursor reaches the last visible row.
        assert_eq!(window_start(6, 20, 7), 0);
        assert_eq!(window_start(7, 20, 7), 1);
        // And stop once the final row is on screen, rather than scrolling past the end.
        assert_eq!(window_start(19, 20, 7), 13);
    }

    #[test]
    fn map_ids_lose_their_grouping_prefix_and_ordering_numbers() {
        assert_eq!(location_display_name("town_02_millhaven"), "Millhaven");
        assert_eq!(location_display_name("town_03_harborgate"), "Harborgate");
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

    #[test]
    fn visible_rows_index_the_window_and_fade_only_edges_that_hide_rows() {
        let fades = |row: (usize, ScrollFades)| (row.0, row.1.top, row.1.bottom);
        // A scrolled window: hidden rows above and below fade the first and last cards.
        assert_eq!(
            visible_rows(2, 10, 4).map(fades).collect::<Vec<_>>(),
            [
                (2, true, false),
                (3, false, false),
                (4, false, false),
                (5, false, true)
            ]
        );
        // The end of the list fades only its top edge.
        assert_eq!(
            visible_rows(6, 10, 4).map(fades).collect::<Vec<_>>(),
            [
                (6, true, false),
                (7, false, false),
                (8, false, false),
                (9, false, false)
            ]
        );
        // A short list shows every row without fades, and a window past the end shows none.
        assert!(visible_rows(0, 3, 7).all(|(_, fade)| !fade.top && !fade.bottom));
        assert_eq!(visible_rows(0, 3, 7).count(), 3);
        assert_eq!(visible_rows(10, 10, 4).count(), 0);
    }
}
