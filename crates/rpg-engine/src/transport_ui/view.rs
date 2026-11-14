//! Ember Atlas overlay view: spawning, per-frame sync, the palette and the footer hints.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::*;

pub(super) const ATLAS_WIDTH: f32 = 1280.0;
pub(super) const ATLAS_HEIGHT: f32 = 720.0;
pub(super) const MAP_WIDTH: f32 = 820.0;
pub(super) const MAP_HEIGHT: f32 = 548.0;
pub(super) const PANEL_WIDTH: f32 = 370.0;

// Ember Atlas palette, in on-screen order. Each value is pinned by
// `atlas_palette_pins_exact_components`; change one only deliberately.
pub(super) const ATLAS_BACKGROUND: Color = Color::srgba(0.025, 0.015, 0.01, 0.985);
pub(super) const MAP_FRAME_BORDER: Color = Color::srgb_u8(151, 89, 38);
pub(super) const MAP_BACKGROUND: Color = Color::srgb_u8(52, 32, 18);
pub(super) const MAP_SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
pub(super) const FOOTER_TEXT: Color = Color::srgb_u8(205, 188, 151);
pub(super) const CURRENT_MARKER: Color = Color::srgb_u8(255, 220, 126);
pub(super) const REGION_MARKER: Color = Color::srgb_u8(224, 171, 84);
pub(super) const MARKER_FILL: Color = Color::srgba(0.055, 0.025, 0.012, 0.92);
pub(super) const SELECTED_LABEL_FILL: Color = Color::srgba(0.08, 0.035, 0.015, 0.94);
pub(super) const LABEL_FILL: Color = Color::srgba(0.07, 0.045, 0.025, 0.82);
pub(super) const LABEL_TEXT: Color = Color::srgb_u8(244, 224, 182);
pub(super) const PANEL_FILL: Color = Color::srgba(0.085, 0.045, 0.022, 0.97);
pub(super) const PANEL_BORDER: Color = Color::srgb_u8(137, 76, 33);
pub(super) const PANEL_SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.65);
pub(super) const PANEL_HEADING_TEXT: Color = Color::srgb_u8(244, 192, 104);
pub(super) const PANEL_COUNT_TEXT: Color = Color::srgb_u8(166, 143, 108);
pub(super) const EMPTY_NOTICE_FILL: Color = Color::srgba(0.15, 0.09, 0.04, 0.7);
pub(super) const EMPTY_NOTICE_BORDER: Color = Color::srgb_u8(98, 71, 44);
pub(super) const EMPTY_NOTICE_TEXT: Color = Color::srgb_u8(196, 177, 143);
pub(super) const ROW_FILL: Color = Color::srgba(0.14, 0.08, 0.035, 0.72);
pub(super) const ROW_BORDER: Color = Color::srgb_u8(96, 67, 40);
pub(super) const ROW_UNAVAILABLE_BAR: Color = Color::srgb_u8(101, 87, 68);
pub(super) const ROW_SELECTED_NAME_TEXT: Color = Color::srgb_u8(255, 232, 183);
pub(super) const ROW_NAME_TEXT: Color = Color::srgb_u8(221, 207, 176);
pub(super) const ROW_UNAVAILABLE_TEXT: Color = Color::srgb_u8(139, 126, 103);
pub(super) const ROW_REGION_TEXT: Color = Color::srgb_u8(151, 132, 99);
pub(super) const DETAILS_FILL: Color = Color::srgba(0.045, 0.025, 0.014, 0.82);
pub(super) const DETAILS_TEXT: Color = Color::srgb_u8(205, 190, 154);
pub(super) const TITLE_TEXT: Color = Color::srgb_u8(246, 184, 88);
pub(super) const SUBTITLE_TEXT: Color = Color::srgb_u8(184, 151, 103);
pub(super) const TAB_FILL: Color = Color::srgba(0.10, 0.06, 0.03, 0.82);
pub(super) const TAB_BORDER: Color = Color::srgb_u8(105, 75, 45);
pub(super) const TAB_TEXT: Color = Color::srgb_u8(219, 200, 164);
pub(super) const TAB_LOCKED_TEXT: Color = Color::srgb_u8(126, 111, 88);
pub(super) const TOKEN_FILL: Color = Color::srgba(0.06, 0.025, 0.01, 0.92);
pub(super) const WAYPOINT_FILL: Color = Color::srgba(0.05, 0.02, 0.01, 0.94);
pub(super) const ROUTE_CASING: Color = Color::srgba(0.055, 0.018, 0.008, 0.88);
pub(super) const SAIL_ACCENT: Color = Color::srgb_u8(77, 205, 211);
pub(super) const FLY_ACCENT: Color = Color::srgb_u8(247, 188, 72);
pub(super) const WARP_ACCENT: Color = Color::srgb_u8(193, 139, 255);
pub(super) const CLOSED_ACCENT: Color = Color::srgb_u8(180, 160, 120);

#[derive(Component)]
pub(super) struct TransportRoot;

#[derive(Component)]
pub(super) struct SelectedAtlasPin {
    base_size: f32,
}

#[derive(SystemParam)]
pub(super) struct TransportOverlayParams<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    root: Res<'w, ScenarioRoot>,
    inventory: Res<'w, ScenarioInventory>,
    state: Res<'w, TransportUiState>,
    domain: Res<'w, TransportDomain>,
    catalog: Res<'w, FieldMenuCatalog>,
    game: Option<Res<'w, GameState>>,
    roots: Query<'w, 's, Entity, With<TransportRoot>>,
    options: Option<Res<'w, PlayerOptions>>,
}

pub(super) fn sync_transport_overlay(mut commands: Commands, params: TransportOverlayParams) {
    let TransportOverlayParams {
        asset_server,
        root,
        inventory,
        state,
        domain,
        catalog,
        game,
        roots,
        options,
    } = params;
    if state.phase == TransportPhase::Closed {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        return;
    }
    if !state.is_changed()
        && !domain.is_changed()
        && game.as_ref().is_none_or(|g| !g.is_changed())
        && options.as_ref().is_none_or(|options| !options.is_changed())
    {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    let (Some(game), Some(transport), Some(font_path)) =
        (game, domain.catalog(), inventory.font.as_ref())
    else {
        return;
    };
    let font = asset_server.load(root.resolve(font_path));
    let fallback = PlayerOptions::default();
    let bindings = &options.as_deref().unwrap_or(&fallback).bindings;
    let map_image = asset_server.load(root.resolve(&transport.map_image));
    let rows = destinations(&state, &domain, &catalog, &game);
    let window = state
        .scroll
        .window("destinations", state.selected, rows.len(), DESTINATION_ROWS);
    let selected = rows.get(state.selected);
    let current_region = game
        .map()
        .current()
        .and_then(|id| domain.region_for_map(id.as_str()));
    let known_regions = transport
        .regions
        .iter()
        .filter(|region| {
            current_region.is_some_and(|current| current.id == region.id)
                || region.maps.iter().any(|map| {
                    RuntimeMapId::try_new(map.id.clone())
                        .is_ok_and(|id| game.map().has_visited(&id))
                })
        })
        .collect::<Vec<_>>();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: px(ATLAS_WIDTH),
                height: px(ATLAS_HEIGHT),
                padding: UiRect::all(px(22)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(ATLAS_BACKGROUND),
            GlobalZIndex(8_000),
            Pickable::IGNORE,
            TransportRoot,
            Name::new("Ember Atlas"),
        ))
        .with_children(|overlay| {
            spawn_atlas_header(overlay, &font, &state, &domain, &game);
            overlay
                .spawn((Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    column_gap: px(18),
                    ..default()
                },))
                .with_children(|body| {
                    body.spawn((
                        Node {
                            position_type: PositionType::Relative,
                            width: px(MAP_WIDTH),
                            height: px(MAP_HEIGHT),
                            overflow: Overflow::clip(),
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(8)),
                            ..default()
                        },
                        ImageNode::new(map_image).with_mode(NodeImageMode::Stretch),
                        BorderColor::all(MAP_FRAME_BORDER),
                        BackgroundColor(MAP_BACKGROUND),
                        BoxShadow::new(MAP_SHADOW, px(0), px(4), px(0), px(10)),
                    ))
                    .with_children(|map| {
                        spawn_route_preview(map, &state, &domain, &game, selected);
                        for region in known_regions {
                            let current = current_region.is_some_and(|r| r.id == region.id);
                            let chosen = selected.is_some_and(|d| d.region_id == region.id);
                            let place_name = rows
                                .iter()
                                .find(|destination| destination.region_id == region.id)
                                .map_or(region.name.as_str(), |destination| {
                                    destination.name.as_str()
                                });
                            spawn_region_marker(
                                map, &font, region, place_name, state.mode, current, chosen,
                            );
                        }
                    });
                    spawn_destination_panel(body, &font, &state, &domain, &rows, window, selected);
                });
            overlay.spawn((
                Text::new(footer(&state, bindings)),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(FOOTER_TEXT),
                TextShadow {
                    offset: Vec2::new(1.0, 1.0),
                    color: Color::BLACK,
                },
            ));
        });
}

pub(super) fn spawn_region_marker(
    map: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    region: &AtlasRegion,
    place_name: &str,
    mode: TravelMode,
    current: bool,
    selected: bool,
) {
    let x = region.position.x.get() as f32;
    let y = region.position.y.get() as f32;
    let accent = mode_accent(mode);
    let marker_color = if selected {
        accent
    } else if current {
        CURRENT_MARKER
    } else {
        REGION_MARKER
    };
    let base_size = if selected {
        30.0
    } else if current {
        26.0
    } else {
        20.0
    };
    let mut marker = map.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: percent(x * 100.0),
            top: percent(y * 100.0),
            width: px(base_size),
            height: px(base_size),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(if selected || current { 3 } else { 2 })),
            border_radius: BorderRadius::all(percent(50)),
            ..default()
        },
        UiTransform::from_xy(percent(-50), percent(-50)),
        BorderColor::all(marker_color),
        BackgroundColor(MARKER_FILL),
        Outline::new(
            px(if selected { 3 } else { 1 }),
            px(1),
            marker_color.with_alpha(if selected { 0.5 } else { 0.24 }),
        ),
        Name::new(format!("Atlas marker: {}", region.name)),
    ));
    if selected {
        marker.insert(SelectedAtlasPin { base_size });
    }
    marker.with_children(|outer| {
        outer.spawn((
            Node {
                width: px(if selected { 10 } else { 7 }),
                height: px(if selected { 10 } else { 7 }),
                border_radius: BorderRadius::all(percent(50)),
                ..default()
            },
            BackgroundColor(marker_color),
        ));
    });

    let label_offset = if y > 0.65 { -38.0 } else { 18.0 };
    let label = if current {
        format!("{place_name} - CURRENT")
    } else {
        place_name.to_owned()
    };
    map.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: percent(x * 100.0),
            top: percent(y * 100.0),
            padding: UiRect::axes(px(7), px(3)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        UiTransform::from_xy(percent(-50), px(label_offset)),
        BackgroundColor(if selected {
            SELECTED_LABEL_FILL
        } else {
            LABEL_FILL
        }),
        BorderColor::all(marker_color.with_alpha(if selected { 0.9 } else { 0.55 })),
        Text::new(label),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(if selected || current { 15.0 } else { 13.0 }),
            ..default()
        },
        TextColor(if selected { accent } else { LABEL_TEXT }),
        TextShadow {
            offset: Vec2::new(1.0, 1.0),
            color: Color::BLACK,
        },
        Name::new(format!("Atlas label: {}", region.name)),
    ));
}

pub(super) fn spawn_destination_panel(
    body: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    state: &TransportUiState,
    domain: &TransportDomain,
    rows: &[TravelDestination],
    window: ScrollWindow,
    selected: Option<&TravelDestination>,
) {
    body.spawn((
        Node {
            width: px(PANEL_WIDTH),
            height: px(MAP_HEIGHT),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(12)),
            row_gap: px(5),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(PANEL_FILL),
        BorderColor::all(PANEL_BORDER),
        BoxShadow::new(PANEL_SHADOW, px(0), px(4), px(0), px(10)),
    ))
    .with_children(|panel| {
        panel
            .spawn(Node {
                width: percent(100),
                height: px(42),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|heading| {
                heading.spawn((
                    Text::new("CHOOSE A DESTINATION"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(19.0),
                        ..default()
                    },
                    TextColor(PANEL_HEADING_TEXT),
                ));
                heading.spawn((
                    Text::new(destination_count_copy(window.first, rows.len())),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(PANEL_COUNT_TEXT),
                ));
            });

        if rows.is_empty() {
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(86),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        padding: UiRect::all(px(12)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(EMPTY_NOTICE_FILL),
                    BorderColor::all(EMPTY_NOTICE_BORDER),
                ))
                .with_children(|empty| {
                    empty.spawn((
                        Text::new(if state.message.is_empty() {
                            "No charted destinations are available."
                        } else {
                            state.message.as_str()
                        }),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(EMPTY_NOTICE_TEXT),
                        TextLayout::justify(Justify::Center),
                    ));
                });
        } else {
            spawn_scroll_list(panel, window, scroll_list_node(5.0), |panel| {
                for index in window.indices() {
                    spawn_destination_row(
                        panel,
                        font,
                        state.mode,
                        &rows[index],
                        index == state.selected,
                        domain,
                    );
                }
            });
        }

        panel.spawn(Node {
            flex_grow: 1.0,
            min_height: px(2),
            ..default()
        });
        spawn_route_details(panel, font, state, domain, selected);
    });
}

pub(super) fn spawn_destination_row(
    panel: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    mode: TravelMode,
    destination: &TravelDestination,
    selected: bool,
    domain: &TransportDomain,
) {
    let accent = mode_accent(mode);
    let available = destination.availability == TravelAvailability::Available;
    panel
        .spawn((
            Node {
                width: percent(100),
                height: px(39),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(px(9)),
                column_gap: px(8),
                border: UiRect::all(px(if selected { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(if selected {
                accent.with_alpha(0.18)
            } else {
                ROW_FILL
            }),
            BorderColor::all(if selected { accent } else { ROW_BORDER }),
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(if selected { 5 } else { 3 }),
                    height: px(if selected { 24 } else { 16 }),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BackgroundColor(if available {
                    if selected {
                        accent
                    } else {
                        accent.with_alpha(0.55)
                    }
                } else {
                    ROW_UNAVAILABLE_BAR
                }),
            ));
            row.spawn((
                Text::new(destination.name.clone()),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(if available {
                    if selected {
                        ROW_SELECTED_NAME_TEXT
                    } else {
                        ROW_NAME_TEXT
                    }
                } else {
                    ROW_UNAVAILABLE_TEXT
                }),
            ));
            row.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            let region = domain
                .region_for_map(&destination.map_id)
                .map_or("Unknown", |region| region.name.as_str());
            row.spawn((
                Text::new(if available { region } else { "LOCKED" }),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(if selected && available {
                    accent
                } else {
                    ROW_REGION_TEXT
                }),
            ));
        });
}

pub(super) fn spawn_route_details(
    panel: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    state: &TransportUiState,
    domain: &TransportDomain,
    selected: Option<&TravelDestination>,
) {
    let accent = mode_accent(state.mode);
    panel
        .spawn((
            Node {
                width: percent(100),
                min_height: px(78),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                padding: UiRect::axes(px(10), px(7)),
                row_gap: px(3),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(DETAILS_FILL),
            BorderColor::all(accent.with_alpha(0.5)),
        ))
        .with_children(|details| {
            details.spawn((
                Text::new("ROUTE DETAILS"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(accent),
            ));
            details.spawn((
                Text::new(route_detail_copy(state, domain, selected)),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(DETAILS_TEXT),
            ));
        });
}

pub(super) fn route_detail_copy(
    state: &TransportUiState,
    domain: &TransportDomain,
    selected: Option<&TravelDestination>,
) -> String {
    selected.map_or_else(
        || {
            if state.message.is_empty() {
                "Select a destination to preview its route.".to_owned()
            } else {
                state.message.clone()
            }
        },
        |destination| {
            let region = domain
                .region_for_map(&destination.map_id)
                .map_or("Unknown region", |region| region.name.as_str());
            let route = destination
                .availability
                .reason()
                .unwrap_or(match state.mode {
                    TravelMode::Sail => "Charted waterway / no encounters",
                    TravelMode::Fly => "Direct flight / no encounters",
                    TravelMode::Warp => "Spell travel / normal MP cost",
                    TravelMode::Closed => "",
                });
            format!("{} / {}\n{}", region, state.mode.label(), route)
        },
    )
}

pub(super) fn destination_count_copy(first: usize, len: usize) -> String {
    if len == 0 {
        "No charted places".to_owned()
    } else if len > DESTINATION_ROWS {
        format!(
            "Showing {}-{} of {} charted places",
            first + 1,
            (first + DESTINATION_ROWS).min(len),
            len
        )
    } else {
        format!("{} charted place{}", len, if len == 1 { "" } else { "s" })
    }
}

pub(super) fn pulse_selected_pin(time: Res<Time>, mut pins: Query<(&SelectedAtlasPin, &mut Node)>) {
    let pulse = (time.elapsed_secs() * 4.5).sin().mul_add(2.0, 2.0);
    for (pin, mut node) in &mut pins {
        let size = pin.base_size + pulse;
        node.width = px(size);
        node.height = px(size);
    }
}

pub(super) fn spawn_atlas_header(
    overlay: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    state: &TransportUiState,
    domain: &TransportDomain,
    game: &GameState,
) {
    overlay
        .spawn(Node {
            width: percent(100),
            height: px(48),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|header| {
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|title| {
                    title.spawn((
                        Text::new("EMBER ATLAS"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(TITLE_TEXT),
                        TextShadow {
                            offset: Vec2::new(1.0, 2.0),
                            color: Color::BLACK,
                        },
                    ));
                    title.spawn((
                        Text::new("Chart a course across the known realm"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(SUBTITLE_TEXT),
                    ));
                });
            header
                .spawn(Node {
                    height: px(40),
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|tabs| {
                    for mode in TravelMode::ALL {
                        let selected = mode == state.mode;
                        let unlocked = match mode {
                            TravelMode::Warp => state.warp.is_some(),
                            _ => domain.mode_unlocked(mode, game.flags()),
                        };
                        let accent = mode_accent(mode);
                        tabs.spawn((
                            Node {
                                width: px(112),
                                height: px(36),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                padding: UiRect::horizontal(px(8)),
                                border: UiRect::all(px(if selected { 2 } else { 1 })),
                                border_radius: BorderRadius::all(px(5)),
                                ..default()
                            },
                            BackgroundColor(if selected {
                                accent.with_alpha(0.22)
                            } else {
                                TAB_FILL
                            }),
                            BorderColor::all(if selected { accent } else { TAB_BORDER }),
                        ))
                        .with_children(|tab| {
                            tab.spawn((
                                Text::new(if unlocked {
                                    mode.label().to_uppercase()
                                } else {
                                    format!("{} LOCKED", mode.label().to_uppercase())
                                }),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(if unlocked { 16.0 } else { 12.0 }),
                                    ..default()
                                },
                                TextColor(if unlocked {
                                    if selected { accent } else { TAB_TEXT }
                                } else {
                                    TAB_LOCKED_TEXT
                                }),
                            ));
                        });
                    }
                });
        });
}

/// The atlas footer: `LEFT/RIGHT MODE   UP/DOWN DESTINATION   ENTER TRAVEL   ESC BACK`.
/// Modes and destinations move through the menu actions, Confirm travels and Back leaves, so
/// the keys follow rebinds.
pub(super) fn footer(state: &TransportUiState, bindings: &InputBindings) -> String {
    const CONFIRM: HintKey = HintKey::Action(AppAction::Confirm);
    const BACK: HintKey = HintKey::Action(AppAction::Back);
    let hint = |segments: &[crate::key_hints::HintSegment<'_>]| {
        key_hint(bindings, Device::Keyboard, &ATLAS_HINTS, segments)
    };
    match state.phase {
        TransportPhase::Browse => format!(
            "{}{}",
            hint(&[
                (&[HintKey::Pair(AppAction::Left, AppAction::Right)], "MODE"),
                (
                    &[HintKey::Pair(AppAction::Up, AppAction::Down)],
                    "DESTINATION"
                ),
                (&[CONFIRM], "TRAVEL"),
                (&[BACK], "BACK"),
            ]),
            if state.message.is_empty() {
                String::new()
            } else {
                format!("     {}", state.message)
            }
        ),
        TransportPhase::Confirm => format!(
            "{}     {}",
            hint(&[(&[CONFIRM], "CONFIRM"), (&[BACK], "BACK")]),
            state.message
        ),
        TransportPhase::Animating => format!(
            "{}   Travel is being prepared...",
            hint(&[(&[CONFIRM], "SKIP ANIMATION")])
        ),
        TransportPhase::Closed => String::new(),
    }
}

/// The atlas footer's separators: a single space between key and label, three between segments.
pub(super) const ATLAS_HINTS: HintStyle = HintStyle {
    key_gap: " ",
    segment_gap: "   ",
    pair: "/",
    either: " / ",
    label_first: false,
};

pub(super) fn spawn_route_preview(
    map: &mut ChildSpawnerCommands,
    state: &TransportUiState,
    domain: &TransportDomain,
    game: &GameState,
    selected: Option<&TravelDestination>,
) {
    let Some(destination) = selected else { return };
    let Some(current) = game
        .map()
        .current()
        .and_then(|id| domain.region_for_map(id.as_str()))
    else {
        return;
    };
    let Some(target) = domain
        .catalog()
        .and_then(|c| c.regions.iter().find(|r| r.id == destination.region_id))
    else {
        return;
    };
    let points = if state.mode == TravelMode::Sail && !destination.path.is_empty() {
        destination
            .path
            .iter()
            .filter_map(|berth_id| {
                let catalog = domain.catalog()?;
                let berth = catalog.sail_berths.iter().find(|b| &b.id == berth_id)?;
                let region = catalog.regions.iter().find(|r| r.id == berth.region)?;
                Some((
                    region.position.x.get() as f32,
                    region.position.y.get() as f32,
                ))
            })
            .collect::<Vec<_>>()
    } else if state.mode == TravelMode::Fly {
        let start = (
            current.position.x.get() as f32,
            current.position.y.get() as f32,
        );
        let end = (
            target.position.x.get() as f32,
            target.position.y.get() as f32,
        );
        let control = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5 - 0.13);
        (0..=12)
            .map(|step| {
                let t = step as f32 / 12.0;
                let one_minus = 1.0 - t;
                (
                    one_minus * one_minus * start.0
                        + 2.0 * one_minus * t * control.0
                        + t * t * end.0,
                    one_minus * one_minus * start.1
                        + 2.0 * one_minus * t * control.1
                        + t * t * end.1,
                )
            })
            .collect()
    } else {
        let start = (
            current.position.x.get() as f32,
            current.position.y.get() as f32,
        );
        let end = (
            target.position.x.get() as f32,
            target.position.y.get() as f32,
        );
        (0..=12)
            .map(|step| {
                let t = step as f32 / 12.0;
                (
                    start.0 + (end.0 - start.0) * t,
                    start.1 + (end.1 - start.1) * t,
                )
            })
            .collect()
    };
    for pair in points.windows(2) {
        spawn_line(map, pair[0], pair[1], state.mode);
    }
    if state.mode == TravelMode::Sail && points.len() > 2 {
        for point in points.iter().skip(1).take(points.len() - 2) {
            spawn_route_waypoint(map, *point, state.mode);
        }
    }
    if state.phase == TransportPhase::Animating {
        let t = (state.animation_elapsed / state.animation_duration.max(0.001)).clamp(0.0, 1.0);
        if state.mode == TravelMode::Warp && points.len() >= 2 {
            let start = points[0];
            let end = points[points.len() - 1];
            let approach = t * 0.5;
            spawn_animation_token(
                map,
                state.mode,
                start.0 + (end.0 - start.0) * approach,
                start.1 + (end.1 - start.1) * approach,
            );
            spawn_animation_token(
                map,
                state.mode,
                end.0 + (start.0 - end.0) * approach,
                end.1 + (start.1 - end.1) * approach,
            );
            return;
        }
        let segment = ((points.len().saturating_sub(1)) as f32 * t).floor() as usize;
        let segment = segment.min(points.len().saturating_sub(2));
        if points.len() >= 2 {
            let local = ((points.len() - 1) as f32 * t) - segment as f32;
            let x = points[segment].0 + (points[segment + 1].0 - points[segment].0) * local;
            let y = points[segment].1 + (points[segment + 1].1 - points[segment].1) * local;
            spawn_animation_token(map, state.mode, x, y);
        }
    }
}

pub(super) fn spawn_animation_token(
    map: &mut ChildSpawnerCommands,
    mode: TravelMode,
    x: f32,
    y: f32,
) {
    let accent = mode_accent(mode);
    let (width, height, radius, rotation) = match mode {
        TravelMode::Sail => (20.0, 11.0, 5.0, Rot2::IDENTITY),
        TravelMode::Fly => (15.0, 15.0, 2.0, Rot2::radians(std::f32::consts::FRAC_PI_4)),
        TravelMode::Warp => (19.0, 19.0, 50.0, Rot2::IDENTITY),
        TravelMode::Closed => return,
    };
    map.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: percent(x * 100.0),
            top: percent(y * 100.0),
            width: px(width),
            height: px(height),
            border: UiRect::all(px(3)),
            border_radius: BorderRadius::all(px(radius)),
            ..default()
        },
        UiTransform {
            translation: Val2::new(percent(-50), percent(-50)),
            rotation,
            ..default()
        },
        BackgroundColor(TOKEN_FILL),
        BorderColor::all(accent),
        Outline::new(px(3), px(1), accent.with_alpha(0.35)),
    ));
}

pub(super) fn spawn_route_waypoint(
    map: &mut ChildSpawnerCommands,
    point: (f32, f32),
    mode: TravelMode,
) {
    let accent = mode_accent(mode);
    map.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: percent(point.0 * 100.0),
            top: percent(point.1 * 100.0),
            width: px(10),
            height: px(10),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(percent(50)),
            ..default()
        },
        UiTransform::from_xy(percent(-50), percent(-50)),
        BackgroundColor(WAYPOINT_FILL),
        BorderColor::all(accent),
    ));
}

pub(super) fn spawn_line(
    map: &mut ChildSpawnerCommands,
    from: (f32, f32),
    to: (f32, f32),
    mode: TravelMode,
) {
    let dx = (to.0 - from.0) * MAP_WIDTH;
    let dy = (to.1 - from.1) * MAP_HEIGHT;
    let dash_scale = match mode {
        TravelMode::Fly => 0.84,
        TravelMode::Warp => 0.55,
        _ => 1.0,
    };
    let length = (dx * dx + dy * dy).sqrt() * dash_scale;
    let midpoint_x = (from.0 + to.0) * MAP_WIDTH * 0.5;
    let midpoint_y = (from.1 + to.1) * MAP_HEIGHT * 0.5;
    let rotation = Rot2::radians(dy.atan2(dx));
    for (thickness, color) in [
        (8.0, ROUTE_CASING),
        (3.0, mode_accent(mode).with_alpha(0.92)),
    ] {
        map.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(midpoint_x - length * 0.5),
                top: px(midpoint_y - thickness * 0.5),
                width: px(length),
                height: px(thickness),
                border_radius: BorderRadius::all(px(thickness * 0.5)),
                ..default()
            },
            BackgroundColor(color),
            UiTransform::from_rotation(rotation),
        ));
    }
}

pub(super) fn mode_accent(mode: TravelMode) -> Color {
    match mode {
        TravelMode::Sail => SAIL_ACCENT,
        TravelMode::Fly => FLY_ACCENT,
        TravelMode::Warp => WARP_ACCENT,
        TravelMode::Closed => CLOSED_ACCENT,
    }
}
