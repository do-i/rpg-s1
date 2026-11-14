//! Dialogue and treasure overlay views: spawning, per-frame text sync and the hint lines.

use bevy::prelude::*;

use super::*;

#[derive(Component)]
pub(super) struct WorldDialogueRoot;

#[derive(Component)]
pub(super) struct TreasureOverlayRoot;

#[derive(Component)]
pub(super) struct WorldDialogueSpeaker;

#[derive(Component)]
pub(super) struct WorldDialogueSpeakerPlate;

#[derive(Component)]
pub(super) struct WorldDialoguePortrait;

#[derive(Component)]
pub(super) struct WorldDialogueBody;

#[derive(Component)]
pub(super) struct WorldDialogueChoices;

/// The `>` gutter beside the choice list, drawn in its own fixed-width column.
///
/// The marker is not a `">"`/`" "` prefix inside the choice text itself: the scenario font
/// is proportional, so `>` and a space are not the same width and every label would shift
/// sideways as the selection moved. Laying the text out at `x + cursor_width` regardless of
/// which row is selected avoids this, and this column is the same idea.
#[derive(Component)]
pub(super) struct WorldDialogueChoiceCursors;

#[derive(Component)]
pub(super) struct WorldDialogueHint;

/// `3/8` — which line of the current entry is on screen. Shares the hint's row.
#[derive(Component)]
pub(super) struct WorldDialoguePageCount;

#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "the dialogue overlay updates disjoint Bevy Text roles in one synchronized pass"
)]
pub(super) fn sync_dialogue_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    theme: Res<UiTheme>,
    state: Res<WorldInteractionState>,
    mut roots: Query<(Entity, &mut Name), With<WorldDialogueRoot>>,
    mut speakers: Query<
        &mut Text,
        (
            With<WorldDialogueSpeaker>,
            Without<WorldDialogueBody>,
            Without<WorldDialogueChoices>,
            Without<WorldDialogueChoiceCursors>,
            Without<WorldDialogueHint>,
            Without<WorldDialoguePageCount>,
        ),
    >,
    mut bodies: Query<
        &mut Text,
        (
            With<WorldDialogueBody>,
            Without<WorldDialogueSpeaker>,
            Without<WorldDialogueChoices>,
            Without<WorldDialogueChoiceCursors>,
            Without<WorldDialogueHint>,
            Without<WorldDialoguePageCount>,
        ),
    >,
    mut choices: Query<
        &mut Text,
        (
            With<WorldDialogueChoices>,
            Without<WorldDialogueSpeaker>,
            Without<WorldDialogueBody>,
            Without<WorldDialogueChoiceCursors>,
            Without<WorldDialogueHint>,
            Without<WorldDialoguePageCount>,
        ),
    >,
    mut choice_cursors: Query<
        &mut Text,
        (
            With<WorldDialogueChoiceCursors>,
            Without<WorldDialogueSpeaker>,
            Without<WorldDialogueBody>,
            Without<WorldDialogueChoices>,
            Without<WorldDialogueHint>,
            Without<WorldDialoguePageCount>,
        ),
    >,
    mut hints: Query<
        &mut Text,
        (
            With<WorldDialogueHint>,
            Without<WorldDialogueSpeaker>,
            Without<WorldDialogueBody>,
            Without<WorldDialogueChoices>,
            Without<WorldDialogueChoiceCursors>,
            Without<WorldDialoguePageCount>,
        ),
    >,
    mut page_counts: Query<
        &mut Text,
        (
            With<WorldDialoguePageCount>,
            Without<WorldDialogueSpeaker>,
            Without<WorldDialogueBody>,
            Without<WorldDialogueChoices>,
            Without<WorldDialogueChoiceCursors>,
            Without<WorldDialogueHint>,
        ),
    >,
    mut plates: Query<&mut Node, With<WorldDialogueSpeakerPlate>>,
    mut portraits: Query<(&mut ImageNode, &mut Visibility), With<WorldDialoguePortrait>>,
    options: Option<Res<PlayerOptions>>,
) {
    let Some(session) = state.session() else {
        for (entity, _) in &mut roots {
            commands.entity(entity).despawn();
        }
        return;
    };
    if roots.is_empty() {
        let Some(font_path) = inventory.font.as_ref() else {
            return;
        };
        let font = asset_server.load(scenario_root.resolve(font_path));
        spawn_dialogue_overlay(&mut commands, &theme, font);
        return;
    }
    let root_name = format!("World dialogue: {}", session.id());
    for (_, mut name) in &mut roots {
        if name.as_str() != root_name {
            name.set(root_name.clone());
        }
    }
    if let Ok(mut speaker) = speakers.single_mut() {
        set_text_if_changed(&mut speaker, session.speaker().unwrap_or_default());
    }
    if let Ok(mut plate) = plates.single_mut() {
        // The source only draws the plate for a named speaker; a sign's box has no name on it.
        plate.display = if session.speaker().is_some_and(|name| !name.is_empty()) {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok((mut portrait, mut visibility)) = portraits.single_mut() {
        match state.portrait.as_ref() {
            Some(source) => {
                *portrait = source.image_node();
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    if let Ok(mut body) = bodies.single_mut() {
        set_text_if_changed(&mut body, &session.visible_text());
    }
    // The gutter and the labels are two columns of the same row count, so the marker can move
    // without ever reflowing a label. Both write one `\n`-joined string into their own node.
    if let Ok(mut cursor_text) = choice_cursors.single_mut() {
        let cursors = session
            .choices()
            .iter()
            .enumerate()
            .map(|(index, _)| {
                if index == session.selected_choice() {
                    ">"
                } else {
                    ""
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        set_text_if_changed(&mut cursor_text, &cursors);
    }
    if let Ok(mut choice_text) = choices.single_mut() {
        let labels = session
            .choices()
            .iter()
            .map(|choice| {
                let disabled = if choice.enabled() { "" } else { " [locked]" };
                format!("{}{disabled}", choice.text())
            })
            .collect::<Vec<_>>()
            .join("\n");
        set_text_if_changed(&mut choice_text, &labels);
    }
    if let Ok(mut page_count) = page_counts.single_mut() {
        let pages = session
            .page()
            .map(|(page, total)| format!("{page}/{total}"))
            .unwrap_or_default();
        set_text_if_changed(&mut page_count, &pages);
    }
    if let Ok(mut hint) = hints.single_mut() {
        let default = PlayerOptions::default();
        let options = options.as_deref().unwrap_or(&default);
        let hint_text = dialogue_hint(
            session.phase(),
            session.finishes_on_confirm(),
            &options.bindings,
        );
        set_text_if_changed(&mut hint, &hint_text);
    }
}

/// The dialogue box's hint line: `ENTER · reveal     ESC · close`. Choosing is navigated with
/// the vertical menu actions, a line is advanced by Confirm and the box closed by Back, so the
/// keys follow rebinds.
pub(super) fn dialogue_hint(
    phase: DialoguePhase,
    finishes_on_confirm: bool,
    bindings: &InputBindings,
) -> String {
    const CONFIRM: HintKey = HintKey::Action(AppAction::Confirm);
    const BACK: HintKey = HintKey::Action(AppAction::Back);
    let hint = |segments: &[crate::key_hints::HintSegment<'_>]| {
        key_hint(bindings, Device::Keyboard, &DIALOGUE_HINTS, segments)
    };
    match phase {
        DialoguePhase::Typing => hint(&[(&[CONFIRM], "reveal"), (&[BACK], "close")]),
        DialoguePhase::Ready if finishes_on_confirm => {
            format!(">  {}", hint(&[(&[CONFIRM], "finish"), (&[BACK], "close")]))
        }
        DialoguePhase::Ready => {
            format!(
                ">  {}",
                hint(&[(&[CONFIRM], "continue"), (&[BACK], "close")])
            )
        }
        DialoguePhase::Choosing => hint(&[
            (&[HintKey::Pair(AppAction::Up, AppAction::Down)], "choose"),
            (&[CONFIRM], "select"),
            (&[BACK], "close"),
        ]),
        DialoguePhase::Closed => String::new(),
    }
}

/// The dialogue hint's separators: a dot between key and label, wide gaps between segments.
const DIALOGUE_HINTS: HintStyle = HintStyle {
    key_gap: " · ",
    segment_gap: "     ",
    pair: "/",
    either: " / ",
    label_first: false,
};

/// The treasure reveal's prompt: `ENTER  take`.
const TREASURE_HINTS: HintStyle = HintStyle {
    key_gap: "  ",
    segment_gap: "  ",
    pair: "/",
    either: " / ",
    label_first: false,
};

/// Assigns through `DerefMut` only when the string differs, so an unchanged line is not laid out
/// again every frame.
fn set_text_if_changed(text: &mut Mut<Text>, value: &str) {
    if text.0 != value {
        text.0 = value.to_owned();
    }
}

/// Box geometry.
///
/// The box is sized from the screen -- `box_w = screen_w - BOX_MARGIN * 2` -- so it always
/// spans the display; a hardcoded width would leave canvas empty to the right of every
/// conversation.
const DIALOGUE_BOX_MARGIN: f32 = 20.0;
const DIALOGUE_BOX_HEIGHT: f32 = 180.0;
const DIALOGUE_BOX_PADDING: f32 = 16.0;
const DIALOGUE_PORTRAIT_SIZE: f32 = 96.0;
/// The speaker plate straddles the box's top border rather than sitting inside it.
const DIALOGUE_PLATE_HEIGHT: f32 = 30.0;
/// Width of the choice-list marker column, wide enough for `>` plus the gap that separates it from the label.
const DIALOGUE_CHOICE_CURSOR_WIDTH: f32 = 18.0;

pub(super) fn spawn_dialogue_overlay(commands: &mut Commands, theme: &UiTheme, font: Handle<Font>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(DIALOGUE_BOX_MARGIN),
                // Both insets rather than a width, so the box tracks the canvas the way the
                // source's `screen.get_width() - BOX_MARGIN * 2` does.
                right: px(DIALOGUE_BOX_MARGIN),
                bottom: px(DIALOGUE_BOX_MARGIN),
                height: px(DIALOGUE_BOX_HEIGHT),
                // The box's 2 px border sits *inside* the rect, so the contents
                // start `BOX_PAD` from the box edge. Bevy lays padding out inside the border, so
                // the border's width comes out of the padding to land on the same pixels.
                padding: UiRect::all(px(DIALOGUE_BOX_PADDING - 2.0)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Row,
                column_gap: px(DIALOGUE_BOX_PADDING),
                ..default()
            },
            BackgroundColor(UiTheme::DIALOGUE_PANEL),
            BorderColor::all(theme.name_entry_border_color),
            GlobalZIndex(5_000),
            Pickable::IGNORE,
            Name::new("World dialogue"),
            WorldDialogueRoot,
        ))
        .with_children(|panel| {
            // Name plate: absolutely placed so it can hang over the box's own top border, which
            // is where it belongs.
            panel
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        // Absolute children sit inside the parent's border, so these insets are
                        // measured from there: 14 px in and half a plate up puts the plate on the
                        // box's top border exactly.
                        left: px(14.0 - 2.0),
                        top: px(-(DIALOGUE_PLATE_HEIGHT / 2.0 + 2.0)),
                        height: px(DIALOGUE_PLATE_HEIGHT),
                        padding: UiRect::axes(px(10), px(4)),
                        border: UiRect::all(px(1)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(24, 22, 40)),
                    BorderColor::all(Color::srgb_u8(160, 160, 100)),
                    WorldDialogueSpeakerPlate,
                ))
                .with_child((
                    Text::new(""),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(theme.scaled(18.0)),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(235, 210, 140)),
                    WorldDialogueSpeaker,
                ));
            // The portrait's frame is drawn whether or not a speaker has a sprite, so a
            // sign's dialogue keeps the same text column as an NPC's.
            panel
                .spawn((
                    Node {
                        width: px(DIALOGUE_PORTRAIT_SIZE),
                        height: px(DIALOGUE_PORTRAIT_SIZE),
                        flex_shrink: 0.0,
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(50, 50, 80)),
                    BorderColor::all(Color::srgb_u8(120, 120, 90)),
                ))
                .with_child((
                    ImageNode::default(),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    Visibility::Hidden,
                    WorldDialoguePortrait,
                ));
            panel
                .spawn(Node {
                    flex_grow: 1.0,
                    // Without this a long line widens the row instead of wrapping inside it.
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    ..default()
                })
                .with_children(|column| {
                    column.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(theme.scaled(22.0)),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(220, 220, 180)),
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        Node {
                            width: percent(100),
                            flex_grow: 1.0,
                            ..default()
                        },
                        WorldDialogueBody,
                    ));
                    column
                        .spawn(Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::FlexStart,
                            ..default()
                        })
                        .with_children(|choices| {
                            choices.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(theme.scaled(18.0)),
                                    ..default()
                                },
                                TextColor(theme.name_entry_input_color),
                                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                                // A definite width is what keeps the labels still: the gutter
                                // occupies the same box whether or not it holds the marker.
                                Node {
                                    width: px(DIALOGUE_CHOICE_CURSOR_WIDTH),
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                                WorldDialogueChoiceCursors,
                            ));
                            choices.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(theme.scaled(18.0)),
                                    ..default()
                                },
                                TextColor(theme.name_entry_input_color),
                                Node {
                                    flex_grow: 1.0,
                                    ..default()
                                },
                                WorldDialogueChoices,
                            ));
                        });
                    // Counter and hint share one line: the box has a fixed height, so giving the
                    // page count a row of its own would come out of the body text.
                    column
                        .spawn(Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            column_gap: px(12),
                            ..default()
                        })
                        .with_children(|footer| {
                            footer.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(theme.scaled(15.0)),
                                    ..default()
                                },
                                TextColor(theme.name_entry_hint_color),
                                WorldDialoguePageCount,
                            ));
                            footer.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.into(),
                                    font_size: FontSize::Px(theme.scaled(15.0)),
                                    ..default()
                                },
                                TextColor(theme.name_entry_hint_color),
                                WorldDialogueHint,
                            ));
                        });
                });
        });
}

/// Panel geometry: a fixed-width panel whose height grows one row at a time.
const TREASURE_MODAL_WIDTH: f32 = 520.0;
const TREASURE_ROW_HEIGHT: f32 = 30.0;
const TREASURE_TITLE_HEIGHT: f32 = 42.0;
const TREASURE_HINT_HEIGHT: f32 = 32.0;
const TREASURE_PADDING: f32 = 20.0;

/// Spawns the reveal when a box opens and despawns it when the player dismisses it.
///
/// The rows never change while the modal is up -- the box is looted once, before the reveal
/// exists -- so this spawns the contents rather than re-synchronizing text every frame.
#[expect(
    clippy::too_many_arguments,
    reason = "the treasure reveal spawns from the box rows, assets, theme and live options"
)]
pub(super) fn sync_treasure_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<WorldInteractionState>,
    theme: Res<UiTheme>,
    roots: Query<Entity, With<TreasureOverlayRoot>>,
    options: Option<Res<PlayerOptions>>,
) {
    let Some(reveal) = state.treasure.as_ref() else {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        return;
    };
    if !roots.is_empty() {
        return;
    }
    let Some(font_path) = inventory.font.as_ref() else {
        return;
    };
    let font: Handle<Font> = asset_server.load(scenario_root.resolve(font_path));
    let default = PlayerOptions::default();
    let options = options.as_deref().unwrap_or(&default);
    spawn_treasure_overlay(&mut commands, &theme, font, &reveal.rows, &options.bindings);
}

pub(super) fn spawn_treasure_overlay(
    commands: &mut Commands,
    theme: &UiTheme,
    font: Handle<Font>,
    rows: &[TreasureRow],
    bindings: &InputBindings,
) {
    commands
        .spawn((
            // The scrim covers the whole canvas rather than a fixed size, so the modal stays
            // centered at any viewport the canvas policy produces.
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            // The reference dim is a straight sRGB blend at alpha 160/255 (0.627). Bevy composites
            // in linear space, where that same alpha reads visibly lighter -- measured on the
            // forest, a 58-value pixel landed at 34 instead of 22. This alpha
            // reproduces the *perceived* darkness instead of its numeric value, the same
            // adjustment the dialogue box already carries (0.94 for 0.863).
            BackgroundColor(UiTheme::SCENE_DIM_BACKDROP),
            GlobalZIndex(5_500),
            Pickable::IGNORE,
            Name::new("Treasure reveal"),
            TreasureOverlayRoot,
        ))
        .with_children(|scrim| {
            scrim
                .spawn((
                    Node {
                        width: px(TREASURE_MODAL_WIDTH),
                        // Same border-inside-the-rect correction as the dialogue box.
                        padding: UiRect::all(px(TREASURE_PADDING - 2.0)),
                        border: UiRect::all(px(2)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(20, 20, 45)),
                    BorderColor::all(Color::srgb_u8(160, 160, 100)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("You found:"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(theme.scaled(24.0)),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(240, 220, 140)),
                        Node {
                            height: px(TREASURE_TITLE_HEIGHT),
                            ..default()
                        },
                    ));
                    for row in rows {
                        panel
                            .spawn(Node {
                                min_height: px(TREASURE_ROW_HEIGHT),
                                align_items: AlignItems::Center,
                                margin: UiRect::left(px(8)),
                                ..default()
                            })
                            .with_children(|line| {
                                if let Some(id) = row.item_id.as_deref() {
                                    crate::item_images::spawn_item_icon(line, id, 28.0);
                                }
                                line.spawn((
                                    Text::new(row.text.clone()),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::Px(theme.scaled(20.0)),
                                        ..default()
                                    },
                                    TextColor(Color::srgb_u8(220, 220, 180)),
                                ));
                            });
                    }
                    panel.spawn((
                        Text::new(key_hint(
                            bindings,
                            Device::Keyboard,
                            &TREASURE_HINTS,
                            &[(&[HintKey::Action(AppAction::Confirm)], "take")],
                        )),
                        TextFont {
                            font: font.into(),
                            font_size: FontSize::Px(theme.scaled(16.0)),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(160, 160, 120)),
                        Node {
                            height: px(TREASURE_HINT_HEIGHT),
                            align_self: AlignSelf::FlexEnd,
                            ..default()
                        },
                    ));
                });
        });
}
