//! A framed viewport with overflow highlights and a draggable scrollbar.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};

use crate::{
    action_input::ActionState,
    gameplay_canvas::pointer::{CanvasPointer, map_cursor},
};

use super::{battle_gold, reward_border};

#[derive(Component)]
pub(in crate::battle) struct RewardScrollList {
    frame: Entity,
}

#[derive(Component)]
pub(super) struct RewardScrollFrame;

#[derive(Component)]
pub(in crate::battle) struct RewardScrollTrack(Entity);

#[derive(Component)]
pub(super) struct RewardScrollThumb(Entity);

#[derive(Component)]
pub(super) struct RewardScrollEdge {
    list: Entity,
    top: bool,
}

pub(super) fn spawn_reward_scroll(
    parent: &mut ChildSpawnerCommands<'_>,
    height: f32,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    spawn_scroll_frame(parent, height, false, content);
}

pub(super) fn spawn_reward_scroll_plain(
    parent: &mut ChildSpawnerCommands<'_>,
    height: f32,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    spawn_scroll_frame(parent, height, true, content);
}

fn spawn_scroll_frame(
    parent: &mut ChildSpawnerCommands<'_>,
    height: f32,
    plain: bool,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    let inset = if plain { 0.0 } else { 8.0 };
    let mut frame = parent.spawn((
        RewardScrollFrame,
        Node {
            width: percent(100),
            height: px(height),
            flex_shrink: 0.0,
            padding: UiRect::all(px(inset)),
            border: UiRect::all(px(if plain { 0.0 } else { 1.0 })),
            border_radius: BorderRadius::all(px(7)),
            column_gap: px(6),
            ..default()
        },
        BorderColor::all(reward_border()),
        BackgroundColor(if plain {
            Color::NONE
        } else {
            Color::srgb_u8(17, 23, 39)
        }),
    ));
    let frame_id = frame.id();
    frame.with_children(|frame| {
        let list = frame
            .spawn((
                RewardScrollList { frame: frame_id },
                ScrollPosition::default(),
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .with_children(content)
            .id();
        frame
            .spawn((
                RewardScrollTrack(list),
                Node {
                    width: px(9),
                    height: percent(100),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(32, 38, 57)),
            ))
            .with_children(|track| {
                track.spawn((
                    RewardScrollThumb(list),
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        border_radius: BorderRadius::all(px(3)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(180, 140, 80)),
                ));
            });
        for top in [true, false] {
            frame.spawn((
                RewardScrollEdge { list, top },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(inset),
                    right: px(inset + 15.0),
                    top: if top { px(inset) } else { Val::Auto },
                    bottom: if top { Val::Auto } else { px(inset) },
                    height: px(8),
                    display: Display::None,
                    border: if top {
                        UiRect::top(px(1))
                    } else {
                        UiRect::bottom(px(1))
                    },
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(17, 23, 39, 210)),
                BorderColor::all(battle_gold()),
                ZIndex(1),
            ));
        }
    });
}

pub(super) fn max_scroll(node: &ComputedNode) -> f32 {
    ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.0)
}

#[derive(SystemParam)]
pub(in crate::battle) struct RewardScrollInput<'w, 's> {
    actions: Res<'w, ActionState>,
    pointer: Option<Res<'w, CanvasPointer>>,
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
    buttons: Option<Res<'w, ButtonInput<MouseButton>>>,
    wheel: MessageReader<'w, 's, MouseWheel>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
}

pub(in crate::battle) fn handle_reward_scroll(
    mut input: RewardScrollInput,
    mut lists: Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &mut ScrollPosition,
        ),
        With<RewardScrollList>,
    >,
    tracks: Query<(&RewardScrollTrack, &ComputedNode, &UiGlobalTransform)>,
    mut dragging: Local<Option<Entity>>,
) {
    let wheel = input
        .wheel
        .read()
        .map(|event| {
            -event.y
                * if event.unit == MouseScrollUnit::Line {
                    40.0
                } else {
                    1.0
                }
        })
        .sum::<f32>();
    if !input
        .buttons
        .as_ref()
        .is_some_and(|buttons| buttons.pressed(MouseButton::Left))
    {
        *dragging = None;
    }
    let Ok((entity, node, transform, mut position)) = lists.single_mut() else {
        return;
    };
    let maximum = max_scroll(node);
    if maximum <= 0.5 {
        position.y = 0.0;
        return;
    }
    if let Some(step) = input.actions.menu_navigation() {
        position.y += step as f32 * 130.0;
    }
    if input
        .pointer
        .as_ref()
        .is_some_and(|pointer| pointer.is_over(node, transform))
    {
        position.y += wheel;
    }
    if let Some(keys) = &input.keys {
        if keys.just_pressed(KeyCode::PageDown) {
            position.y += node.size().y * node.inverse_scale_factor();
        }
        if keys.just_pressed(KeyCode::PageUp) {
            position.y -= node.size().y * node.inverse_scale_factor();
        }
        if keys.just_pressed(KeyCode::Home) {
            position.y = 0.0;
        }
        if keys.just_pressed(KeyCode::End) {
            position.y = maximum;
        }
    }
    for (track, track_node, track_transform) in &tracks {
        if track.0 != entity {
            continue;
        }
        if input.pointer.as_ref().is_some_and(|pointer| {
            pointer.just_clicked() && pointer.is_over(track_node, track_transform)
        }) {
            *dragging = Some(entity);
        }
        if *dragging == Some(entity)
            && let Ok(window) = input.windows.single()
            && let Some(cursor) = window.cursor_position().and_then(|cursor| {
                map_cursor(window.physical_size(), window.scale_factor(), cursor)
            })
        {
            let local = track_transform
                .affine()
                .inverse()
                .transform_point2(cursor.viewport);
            let thumb_ratio = (node.size().y / node.content_size().y).clamp(0.0, 1.0);
            let available = track_node.size().y * (1.0 - thumb_ratio);
            if available > 0.0 {
                let fraction = (local.y + track_node.size().y * 0.5
                    - track_node.size().y * thumb_ratio * 0.5)
                    / available;
                position.y = fraction * maximum;
            }
        }
    }
    position.y = position.y.clamp(0.0, maximum);
}

#[derive(QueryData)]
#[query_data(mutable)]
struct RewardScrollPart {
    style: &'static mut Node,
    track: Option<&'static RewardScrollTrack>,
    thumb: Option<&'static RewardScrollThumb>,
    edge: Option<&'static RewardScrollEdge>,
}

type RewardScrollPartFilter = Or<(
    With<RewardScrollTrack>,
    With<RewardScrollThumb>,
    With<RewardScrollEdge>,
)>;

#[derive(SystemParam)]
pub(in crate::battle) struct RewardScrollChrome<'w, 's> {
    lists: Query<
        'w,
        's,
        (
            &'static ComputedNode,
            &'static ScrollPosition,
            &'static RewardScrollList,
        ),
    >,
    frames: Query<'w, 's, &'static mut BorderColor, With<RewardScrollFrame>>,
    parts: Query<'w, 's, RewardScrollPart, RewardScrollPartFilter>,
}

pub(in crate::battle) fn update_reward_scroll_chrome(mut chrome: RewardScrollChrome) {
    for (node, _, list) in &chrome.lists {
        if let Ok(mut border) = chrome.frames.get_mut(list.frame) {
            border.set_all(if max_scroll(node) > 0.5 {
                battle_gold()
            } else {
                reward_border()
            });
        }
    }
    for part in &mut chrome.parts {
        let (mut style, track, thumb, edge) = (part.style, part.track, part.thumb, part.edge);
        let list_id = track
            .map(|part| part.0)
            .or_else(|| thumb.map(|part| part.0))
            .or_else(|| edge.map(|part| part.list))
            .unwrap();
        let Ok((node, position, _)) = chrome.lists.get(list_id) else {
            continue;
        };
        let maximum = max_scroll(node);
        let scrollable = maximum > 0.5;
        if track.is_some() {
            style.display = if scrollable {
                Display::Flex
            } else {
                Display::None
            };
        }
        if thumb.is_some() {
            let ratio = if scrollable {
                (node.size().y / node.content_size().y).clamp(0.0, 1.0)
            } else {
                1.0
            };
            style.height = percent(ratio * 100.0);
            style.top = percent(if scrollable {
                position.y.clamp(0.0, maximum) / maximum * (1.0 - ratio) * 100.0
            } else {
                0.0
            });
        }
        if let Some(edge) = edge {
            let hidden = if edge.top {
                position.y > 0.5
            } else {
                position.y < maximum - 0.5
            };
            style.display = if scrollable && hidden {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}
