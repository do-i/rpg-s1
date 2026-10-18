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

/// Rows kept between the cursor and a window edge that hides more rows, so the next row is always
/// in view before the cursor reaches it, and a scroll arrow never sits over the selected card.
const SCROLL_MARGIN: usize = 1;

/// First visible index of a scrolled list, moved from `previous` only as far as `selected` needs.
///
/// The cursor travels freely inside the window; the window follows only once the cursor comes
/// within [`SCROLL_MARGIN`] rows of an edge that still hides rows. At the true ends of the list the
/// cursor walks onto the first and last rows.
pub(crate) fn scroll_first(previous: usize, selected: usize, len: usize, rows: usize) -> usize {
    if rows == 0 || len <= rows {
        return 0;
    }
    let selected = selected.min(len - 1);
    let margin = SCROLL_MARGIN.min((rows - 1) / 2);
    let lowest = (selected + margin + 1).saturating_sub(rows);
    let highest = selected.saturating_sub(margin);
    previous.clamp(lowest, highest).min(len - rows)
}

/// The visible slice of a scrolled list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ScrollWindow {
    pub(crate) first: usize,
    pub(crate) len: usize,
    pub(crate) rows: usize,
}

impl ScrollWindow {
    /// Indices of the rows on screen.
    pub(crate) fn indices(self) -> std::ops::Range<usize> {
        self.first.min(self.len)..(self.first + self.rows).min(self.len)
    }

    pub(crate) const fn scrolls(self) -> bool {
        self.len > self.rows
    }

    const fn hides_above(self) -> bool {
        self.first > 0
    }

    const fn hides_below(self) -> bool {
        self.first + self.rows < self.len
    }
}

/// Where each windowed list of one screen was last scrolled, keyed by a name per list.
///
/// Free cursor travel needs the previous window, but the menus redraw from `&State`. The memory
/// is a view cache rather than menu state, so it updates behind a lock without marking the
/// state changed and triggering another redraw.
#[derive(Debug, Default)]
pub(crate) struct ScrollMemory(std::sync::Mutex<std::collections::BTreeMap<&'static str, usize>>);

impl ScrollMemory {
    /// The window for `list` with `selected` in view, remembered for the next redraw.
    pub(crate) fn window(
        &self,
        list: &'static str,
        selected: usize,
        len: usize,
        rows: usize,
    ) -> ScrollWindow {
        let mut firsts = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let first = scroll_first(firsts.get(list).copied().unwrap_or(0), selected, len, rows);
        firsts.insert(list, first);
        ScrollWindow { first, len, rows }
    }
}

/// The scroll indicators a windowed list draws: arrow tabs on edges that hide rows, and the
/// scrollbar thumb.
#[derive(Clone, Copy, Component, Debug, Eq, PartialEq)]
pub(crate) enum ScrollIndicator {
    Up,
    Down,
    Thumb,
}

/// Width kept free on a scroll list's right for its scrollbar, so cards never sit under it.
const SCROLL_GUTTER: f32 = 10.0;
const SCROLL_ARROW_TAB_WIDTH: f32 = 34.0;
const SCROLL_ARROW_TAB_HEIGHT: f32 = 14.0;

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

/// A plain column for a scroll list's cards, spaced `gap` apart.
pub(crate) fn scroll_list_node(gap: f32) -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.0,
        row_gap: px(gap),
        ..default()
    }
}

/// Spawns a windowed list's cards inside `node`, with its scroll indicators laid over them.
///
/// While rows are hidden, an arrow tab straddles each edge that hides them and a thin scrollbar
/// on the right shows where the window sits. The cards themselves are never shaded, so the
/// selected card always reads at full strength. The right padding of `node` is replaced by the
/// scrollbar's gutter.
pub(crate) fn spawn_scroll_list(
    parent: &mut ChildSpawnerCommands<'_>,
    window: ScrollWindow,
    node: Node,
    spawn_rows: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    let padding = UiRect {
        right: px(SCROLL_GUTTER),
        ..node.padding
    };
    parent
        .spawn((Node { padding, ..node }, Name::new("Scroll list")))
        .with_children(|list| {
            spawn_rows(list);
            if !window.scrolls() {
                return;
            }
            spawn_scrollbar(list, window);
            if window.hides_above() {
                spawn_scroll_arrow(list, ScrollIndicator::Up);
            }
            if window.hides_below() {
                spawn_scroll_arrow(list, ScrollIndicator::Down);
            }
        });
}

fn spawn_scrollbar(parent: &mut ChildSpawnerCommands<'_>, window: ScrollWindow) {
    let share = |rows: usize| rows as f32 / window.len as f32 * 100.0;
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(2),
                bottom: px(2),
                right: px(1),
                width: px(3),
                border_radius: BorderRadius::all(px(2)),
                ..default()
            },
            BackgroundColor(status_faint().with_alpha(0.3)),
            Name::new("Scrollbar"),
        ))
        .with_children(|track| {
            track.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    top: percent(share(window.first)),
                    height: percent(share(window.rows)),
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BackgroundColor(status_gold()),
                ScrollIndicator::Thumb,
            ));
        });
}

fn spawn_scroll_arrow(parent: &mut ChildSpawnerCommands<'_>, arrow: ScrollIndicator) {
    let up = arrow == ScrollIndicator::Up;
    // The tab is centred on the list's edge, half over the outermost card -- which the scroll
    // margin guarantees is never the selected one.
    let overhang = px(-SCROLL_ARROW_TAB_HEIGHT / 2.0);
    let (top, bottom) = if up {
        (overhang, auto())
    } else {
        (auto(), overhang)
    };
    parent
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(SCROLL_GUTTER),
            top,
            bottom,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(SCROLL_ARROW_TAB_WIDTH),
                    height: px(SCROLL_ARROW_TAB_HEIGHT),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(SCROLL_ARROW_TAB_HEIGHT / 2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(22, 22, 28, 240)),
                BorderColor::all(status_border_active()),
                arrow,
            ))
            .with_children(|tab| {
                // A square with two bordered sides, turned so its corner points the scroll way.
                // The font has no arrow glyphs, so the chevron is drawn rather than typed.
                tab.spawn((
                    Node {
                        width: px(7),
                        height: px(7),
                        border: UiRect {
                            left: px(2),
                            top: px(2),
                            ..default()
                        },
                        ..default()
                    },
                    BorderColor::all(status_gold()),
                    UiTransform {
                        translation: Val2::px(0.0, if up { 2.0 } else { -2.0 }),
                        rotation: Rot2::degrees(if up { 45.0 } else { 225.0 }),
                        ..UiTransform::IDENTITY
                    },
                ));
            });
        });
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
    use super::{ScrollMemory, ScrollWindow, location_display_name, meter_percent, scroll_first};

    #[test]
    fn the_cursor_moves_freely_and_the_window_follows_one_row_from_a_hiding_edge() {
        // Ten rows in a window of four: walking down holds the window still until the cursor
        // reaches the row above the bottom edge, then scrolls one row per step.
        let mut first = 0;
        let mut walk = Vec::new();
        for selected in 0..10 {
            first = scroll_first(first, selected, 10, 4);
            walk.push(first);
        }
        assert_eq!(walk, [0, 0, 0, 1, 2, 3, 4, 5, 6, 6]);
        // Walking back up from the bottom moves the cursor, not the window, until it nears the top.
        assert_eq!(scroll_first(6, 8, 10, 4), 6);
        assert_eq!(scroll_first(6, 7, 10, 4), 6);
        assert_eq!(scroll_first(6, 6, 10, 4), 5);
        // A stale window snaps to wherever the cursor now is.
        assert_eq!(scroll_first(6, 0, 10, 4), 0);
        assert_eq!(scroll_first(0, 9, 10, 4), 6);
        // Short lists never scroll.
        assert_eq!(scroll_first(3, 2, 3, 7), 0);
    }

    #[test]
    fn the_selected_row_never_sits_on_an_edge_that_hides_rows() {
        for len in 0..20 {
            for rows in 1..8 {
                let mut first = 0;
                for selected in (0..len).chain((0..len).rev()) {
                    first = scroll_first(first, selected, len, rows);
                    let window = ScrollWindow { first, len, rows };
                    assert!(
                        window.indices().contains(&selected),
                        "{len}/{rows}/{selected}"
                    );
                    assert!(window.indices().len() == rows.min(len));
                    if rows >= 3 {
                        assert!(!(window.hides_above() && selected == first));
                        assert!(!(window.hides_below() && selected + 1 == first + rows));
                    }
                }
            }
        }
    }

    #[test]
    fn scroll_memory_keeps_each_list_apart() {
        let memory = ScrollMemory::default();
        assert_eq!(memory.window("a", 9, 10, 4).first, 6);
        assert_eq!(memory.window("b", 0, 10, 4).first, 0);
        assert_eq!(memory.window("a", 7, 10, 4).first, 6);
    }

    #[test]
    fn meter_fill_clamps_to_the_track_and_survives_a_zero_maximum() {
        assert_eq!(meter_percent(3, 0), 0.0);
        assert_eq!(meter_percent(25, 100), 25.0);
        assert_eq!(meter_percent(125, 100), 100.0);
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
}
