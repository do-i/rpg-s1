//! Fixed battle deck: party and commands, seven groups, and the shared scrolling list.

use bevy::{
    asset::RenderAssetUsages,
    input::mouse::{MouseScrollUnit, MouseWheel},
};
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

use super::*;
use crate::{
    menu_chrome::{ScrollIndicator, ScrollMemory, scroll_list_node, spawn_scroll_list},
    scenario_class::{Ability, AbilityElement, AbilityKind, BuffEffect, UtilityAbility},
};

pub(super) const GROUP_FRAME_WIDTH: f32 = 190.0;
pub(super) const SPELL_FRAME_WIDTH: f32 = 280.0;
/// Change only these alpha values to calibrate the two translucent surfaces.
pub(super) const LOG_OVERLAY_OPACITY: f32 = 0.60;
const DIAMOND_OPACITY: f32 = 0.50;
const LOG_WIDTH: f32 = 282.0;
const VISIBLE_ROWS: usize = 4;
const ROW_HEIGHT: f32 = 61.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum SpellGroup {
    #[default]
    Fire,
    Water,
    Wind,
    Earth,
    Holy,
    Heal,
    Utility,
}

impl SpellGroup {
    pub(super) const ALL: [Self; 7] = [
        Self::Fire,
        Self::Water,
        Self::Wind,
        Self::Earth,
        Self::Holy,
        Self::Heal,
        Self::Utility,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Fire => "Fire",
            Self::Water => "Water",
            Self::Wind => "Wind",
            Self::Earth => "Earth",
            Self::Holy => "Holy",
            Self::Heal => "Heal",
            Self::Utility => "Utility",
        }
    }

    pub(super) fn of(ability: &Ability) -> Self {
        // Healing is a purpose, including holy healing; it must never disappear into Holy.
        let element = match &ability.kind {
            AbilityKind::Heal(_) => return Self::Heal,
            AbilityKind::Utility(UtilityAbility::RemoveStatus { .. }) => return Self::Heal,
            AbilityKind::Buff(value) if matches!(value.effect, BuffEffect::Regeneration(_)) => {
                return Self::Heal;
            }
            AbilityKind::Spell(value) => Some(value.element),
            AbilityKind::Buff(value) => value.element,
            _ => None,
        };
        match element {
            Some(AbilityElement::Fire) => Self::Fire,
            Some(AbilityElement::Water) => Self::Water,
            Some(AbilityElement::Wind) => Self::Wind,
            Some(AbilityElement::Earth) => Self::Earth,
            Some(AbilityElement::Holy) => Self::Holy,
            None => Self::Utility,
        }
    }
}

#[derive(Default, Resource)]
pub(super) struct BattleDeck {
    pub(super) group: SpellGroup,
    pub(super) list_focused: bool,
    /// 0 = left, 1 = right, 2 = hidden. History continues recording in every position.
    pub(super) log_position: u8,
    pointer_confirm: bool,
    scroll: ScrollMemory,
}

impl BattleDeck {
    pub(super) fn choices(&self, state: &BattleState) -> Vec<usize> {
        state
            .battle_ability_indices()
            .into_iter()
            .filter(|&index| {
                state
                    .active()
                    .is_some_and(|actor| SpellGroup::of(&actor.abilities[index]) == self.group)
            })
            .collect()
    }

    pub(super) fn enabled(state: &BattleState, group: SpellGroup) -> bool {
        state.battle_ability_indices().into_iter().any(|index| {
            state
                .active()
                .is_some_and(|actor| SpellGroup::of(&actor.abilities[index]) == group)
        })
    }

    pub(super) fn open(&mut self, state: &mut BattleState) {
        self.list_focused = false;
        self.scroll = ScrollMemory::default();
        self.group = SpellGroup::ALL
            .into_iter()
            .find(|&group| Self::enabled(state, group))
            .unwrap_or_default();
        self.select_first(state);
    }

    fn select_first(&self, state: &mut BattleState) {
        if let Some(first) = self.choices(state).first() {
            state.ability_index = state
                .battle_ability_indices()
                .iter()
                .position(|index| index == first)
                .unwrap_or(0);
        }
    }

    pub(super) fn move_group(&mut self, state: &mut BattleState, delta: isize) {
        let enabled: Vec<_> = SpellGroup::ALL
            .into_iter()
            .filter(|&group| Self::enabled(state, group))
            .collect();
        if enabled.is_empty() {
            return;
        }
        let current = enabled
            .iter()
            .position(|group| *group == self.group)
            .unwrap_or(0);
        self.group = enabled[wrap_index(current, delta, enabled.len())];
        self.scroll = ScrollMemory::default();
        self.select_first(state);
    }

    pub(super) fn move_spell(&self, state: &mut BattleState, delta: isize) {
        let choices = self.choices(state);
        if choices.is_empty() {
            return;
        }
        let current = self.selected(state, &choices);
        if let Some(next) = choices.get(wrap_index(current, delta, choices.len())) {
            state.ability_index = state
                .battle_ability_indices()
                .iter()
                .position(|index| index == next)
                .unwrap_or(0);
        }
    }

    fn selected(&self, state: &BattleState, choices: &[usize]) -> usize {
        let all = state.battle_ability_indices();
        all.get(state.ability_index)
            .and_then(|selected| choices.iter().position(|index| index == selected))
            .unwrap_or(0)
    }

    pub(super) fn take_pointer_confirm(&mut self) -> bool {
        std::mem::take(&mut self.pointer_confirm)
    }
}

#[derive(Component)]
pub(super) struct GroupButton {
    group: SpellGroup,
    normal: Handle<Image>,
    selected: Handle<Image>,
    disabled: Handle<Image>,
}
#[derive(Component)]
pub(super) struct GroupLabel(SpellGroup);
#[derive(Component)]
pub(super) struct GroupFrame;
#[derive(Component)]
pub(super) struct Picker;
#[derive(Component)]
pub(super) struct PickerRow(usize);
#[derive(Component)]
pub(super) struct LogOverlay;
#[derive(Component)]
pub(super) struct PartyLevel(pub(super) usize);
#[derive(Component)]
pub(super) struct ExperienceFill(usize);
#[derive(Component)]
pub(super) struct ExperienceText(usize);

/// A small procedural UI texture, so rounded diamonds remain code-native geometry.
fn diamond_image(selected: bool, enabled: bool) -> Image {
    const SCALE: u32 = 4;
    let width = 130 * SCALE;
    let height = 44 * SCALE;
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    let vertices = [
        Vec2::new(0.0, -17.0),
        Vec2::new(59.0, 0.0),
        Vec2::new(0.0, 17.0),
        Vec2::new(-59.0, 0.0),
    ];
    for y in 0..height {
        for x in 0..width {
            let p = Vec2::new(
                (x as f32 + 0.5) / SCALE as f32 - 65.0,
                (y as f32 + 0.5) / SCALE as f32 - 22.0,
            );
            let mut distance = f32::MAX;
            for edge in 0..4 {
                let a = vertices[edge];
                let ab = vertices[(edge + 1) % 4] - a;
                let closest = a + ab * ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
                distance = distance.min(p.distance(closest));
            }
            if p.x.abs() / 59.0 + p.y.abs() / 17.0 <= 1.0 {
                distance = -distance;
            }
            distance -= 3.0; // Round the four corners without changing the text's shape.
            let coverage = (0.5 - distance * SCALE as f32).clamp(0.0, 1.0);
            let border = distance > -1.0;
            let rgb = if border {
                if selected {
                    [235, 190, 89]
                } else {
                    [177, 138, 80]
                }
            } else {
                [30, 30, 38]
            };
            let opacity = if border {
                if enabled { 1.0 } else { 0.28 }
            } else {
                DIAMOND_OPACITY
            };
            data.extend_from_slice(&[
                rgb[0],
                rgb[1],
                rgb[2],
                (255.0 * coverage * opacity).round() as u8,
            ]);
        }
    }
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "the deck shares the battle's existing content and asset sources"
)]
pub(super) fn spawn_panels(
    parent: &mut ChildSpawnerCommands<'_>,
    entry: &BattleEntry,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    font: &Handle<Font>,
    images: &mut Assets<Image>,
) {
    let party: Vec<_> = entry
        .participants
        .iter()
        .filter(|actor| actor.side == BattleSide::Party)
        .collect();
    let normal = images.add(diamond_image(false, true));
    let selected = images.add(diamond_image(true, true));
    let disabled = images.add(diamond_image(false, false));
    let background = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("media/images/ui/battle_spell_groups.png")
                .expect("static UI path"),
        ),
    );
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(298),
            padding: UiRect::all(px(8)),
            column_gap: px(8),
            ..default()
        })
        .with_children(|bottom| {
            bottom
                .spawn(Node {
                    width: px(party_panel_width()),
                    height: percent(100),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                })
                .with_children(|left| {
                    left.spawn(Node {
                        width: percent(100),
                        flex_grow: 1.0,
                        ..default()
                    })
                    .with_children(|party_row| {
                        spawn_party_cards(
                            party_row,
                            &party,
                            game,
                            catalog,
                            asset_server,
                            root,
                            font,
                        );
                    });
                    spawn_command_rows(left, font);
                });
            bottom
                .spawn((
                    Node {
                        width: px(GROUP_FRAME_WIDTH),
                        height: percent(100),
                        flex_shrink: 0.0,
                        position_type: PositionType::Relative,
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(6)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    ImageNode::new(background).with_mode(NodeImageMode::Stretch),
                    BorderColor::all(battle_border()),
                    GroupFrame,
                ))
                .with_children(|groups| {
                    for (index, group) in SpellGroup::ALL.into_iter().enumerate() {
                        groups
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(if index % 2 == 0 { 14.0 } else { 46.0 }),
                                    top: px(8.0 + index as f32 * 37.0),
                                    width: px(130),
                                    height: px(44),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                ImageNode::new(normal.clone()),
                                GroupButton {
                                    group,
                                    normal: normal.clone(),
                                    selected: selected.clone(),
                                    disabled: disabled.clone(),
                                },
                            ))
                            .with_children(|diamond| {
                                spawn_battle_text(
                                    diamond,
                                    group.label(),
                                    font,
                                    16.0,
                                    battle_ink(),
                                    Justify::Center,
                                )
                                .insert(GroupLabel(group));
                            });
                    }
                });
            bottom.spawn((
                Node {
                    width: px(SPELL_FRAME_WIDTH),
                    height: percent(100),
                    flex_shrink: 0.0,
                    padding: UiRect::all(px(8)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(battle_panel().with_alpha(1.0)),
                BorderColor::all(battle_border()),
                Picker,
            ));
        });
}

pub(super) fn spawn_log_overlay(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: px(LOG_WIDTH),
                height: px(ENEMY_AREA_HEIGHT),
                padding: UiRect::axes(px(15), px(13)),
                ..default()
            },
            BackgroundColor(Color::srgba(
                22.0 / 255.0,
                22.0 / 255.0,
                28.0 / 255.0,
                LOG_OVERLAY_OPACITY,
            )),
            ZIndex(2),
            LogOverlay,
        ))
        .with_children(spawn_message_log);
    // Keep prompts visible even when the player hides the history overlay.
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(10),
                bottom: px(298),
                width: px(485),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            ZIndex(3),
        ))
        .with_children(|prompt| {
            spawn_battle_text(prompt, "", font, 12.0, battle_ink(), Justify::Right)
                .insert(BattleMessageText);
            spawn_battle_text(prompt, "", font, 12.0, battle_violet(), Justify::Right)
                .insert(BattleTargetText);
            spawn_battle_text(
                prompt,
                "L  LOG: LEFT / RIGHT / HIDE",
                font,
                10.0,
                battle_ink(),
                Justify::Right,
            );
        });
}

pub(super) fn sync_log_overlay(
    deck: Option<Res<BattleDeck>>,
    mut overlays: Query<&mut Node, With<LogOverlay>>,
) {
    let Some(deck) = deck else { return };
    if !deck.is_changed() {
        return;
    }
    for mut node in &mut overlays {
        node.display = if deck.log_position == 2 {
            Display::None
        } else {
            Display::Flex
        };
        node.left = if deck.log_position == 1 {
            Val::Auto
        } else {
            px(0)
        };
        node.right = if deck.log_position == 1 {
            px(0)
        } else {
            Val::Auto
        };
    }
}

pub(super) fn sync_groups(
    state: Option<Res<BattleState>>,
    deck: Option<Res<BattleDeck>>,
    mut groups: Query<(&GroupButton, &mut ImageNode)>,
    mut labels: Query<(&GroupLabel, &mut TextColor)>,
    mut frames: Query<&mut BorderColor, With<GroupFrame>>,
) {
    let (Some(state), Some(deck)) = (state, deck) else {
        return;
    };
    if !state.is_changed() && !deck.is_changed() {
        return;
    }
    let choosing = state.phase == BattlePhase::Ability
        || (state.phase == BattlePhase::Target && state.pending_ability.is_some());
    for (marker, mut image) in &mut groups {
        image.image = if !BattleDeck::enabled(&state, marker.group) {
            marker.disabled.clone()
        } else if choosing && deck.group == marker.group {
            marker.selected.clone()
        } else {
            marker.normal.clone()
        };
    }
    for (marker, mut color) in &mut labels {
        color.0 = battle_ink().with_alpha(if BattleDeck::enabled(&state, marker.0) {
            1.0
        } else {
            0.28
        });
    }
    for mut border in &mut frames {
        border.set_all(
            if state.phase == BattlePhase::Ability && !deck.list_focused {
                battle_border_active()
            } else {
                battle_border()
            },
        );
    }
}

pub(super) fn sync_picker(
    mut commands: Commands,
    state: Option<Res<BattleState>>,
    deck: Option<Res<BattleDeck>>,
    assets: Option<Res<BattleAssetState>>,
    pickers: Query<(Entity, Option<&Children>), With<Picker>>,
    mut borders: Query<&mut BorderColor, With<Picker>>,
) {
    let (Some(state), Some(deck), Some(assets)) = (state, deck, assets) else {
        return;
    };
    if !state.is_changed() && !deck.is_changed() {
        return;
    }
    let abilities = state.phase == BattlePhase::Ability
        || (state.phase == BattlePhase::Target && state.pending_ability.is_some());
    let items = state.phase == BattlePhase::Item
        || (state.phase == BattlePhase::Target && state.pending_item.is_some());
    for mut border in &mut borders {
        border.set_all(
            if state.phase == BattlePhase::Ability && deck.list_focused
                || state.phase == BattlePhase::Item
            {
                battle_border_active()
            } else {
                battle_border()
            },
        );
    }
    for (picker, children) in &pickers {
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }
        commands.entity(picker).with_children(|panel| {
            if !abilities && !items {
                let message = match state.phase {
                    BattlePhase::Command => "Choose an action.",
                    BattlePhase::Victory | BattlePhase::Rewards => "Victory",
                    BattlePhase::Target => "Choose a target.",
                    _ => "",
                };
                spawn_battle_text(
                    panel,
                    message,
                    &assets.font,
                    16.0,
                    battle_ink(),
                    Justify::Left,
                );
                return;
            }
            let choices = deck.choices(&state);
            let (selected, len) = if abilities {
                (deck.selected(&state, &choices), choices.len())
            } else {
                (state.item_index, state.item_choices.len())
            };
            let window = deck.scroll.window(
                if abilities {
                    "battle-spells"
                } else {
                    "battle-items"
                },
                selected,
                len,
                VISIBLE_ROWS,
            );
            spawn_scroll_list(panel, window, scroll_list_node(6.0), |list| {
                for index in window.indices() {
                    if abilities {
                        if let Some(ability) = choices
                            .get(index)
                            .and_then(|&i| state.active()?.abilities.get(i))
                        {
                            spawn_ability_row(
                                list,
                                ability,
                                index,
                                selected == index && deck.list_focused,
                                state
                                    .active()
                                    .is_some_and(|actor| actor.mana >= ability.mp_cost),
                                &assets.font,
                            );
                        }
                    } else if let Some(item) = state.item_choices.get(index) {
                        spawn_item_row(list, item, index, selected == index, &assets.font);
                    }
                }
            });
        });
    }
}

fn row_node() -> Node {
    Node {
        width: percent(100),
        height: px(ROW_HEIGHT),
        flex_shrink: 0.0,
        padding: UiRect::all(px(4)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(4)),
        column_gap: px(6),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn target_label(ability: &Ability) -> &'static str {
    if let AbilityKind::Spell(value) = &ability.kind {
        match value.target {
            crate::scenario_class::AbilityTarget::AllEnemies => return "All enemies",
            crate::scenario_class::AbilityTarget::GroupEnemies => return "Enemy group",
            _ => {}
        }
    }
    match target_plan(ability) {
        AbilityTargetPlan::Select {
            group: TargetGroup::Enemy,
            ..
        } => "One enemy",
        AbilityTargetPlan::Select { .. } => "One ally",
        AbilityTargetPlan::All {
            side: BattleSide::Enemy,
            ..
        } => "Enemy group",
        AbilityTargetPlan::All { .. } => "All allies",
        AbilityTargetPlan::SelfTarget => "Self",
    }
}

fn spawn_ability_row(
    parent: &mut ChildSpawnerCommands<'_>,
    ability: &Ability,
    index: usize,
    selected: bool,
    affordable: bool,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            row_node(),
            BackgroundColor(if selected {
                battle_row_active()
            } else {
                battle_row()
            }),
            BorderColor::all(if selected {
                battle_border_active()
            } else {
                battle_row_border()
            }),
            PickerRow(index),
        ))
        .with_children(|row| {
            row.spawn((
                SpellIcon::new(&ability.id),
                Node {
                    width: px(ABILITY_ICON_SIZE),
                    height: px(ABILITY_ICON_SIZE),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            row.spawn(Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|label| {
                spawn_battle_text(
                    label,
                    &ability.name,
                    font,
                    18.0,
                    if affordable {
                        battle_ink()
                    } else {
                        battle_ink().with_alpha(0.55)
                    },
                    Justify::Left,
                )
                .insert(TextLayout::new(Justify::Left, LineBreak::NoWrap));
                spawn_battle_text(
                    label,
                    target_label(ability),
                    font,
                    12.0,
                    Color::srgb_u8(170, 160, 142),
                    Justify::Left,
                );
            });
            row.spawn(Node {
                width: px(50),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                column_gap: px(3),
                ..default()
            })
            .with_children(|mp| {
                spawn_battle_text(
                    mp,
                    "MP",
                    font,
                    16.0,
                    Color::srgb_u8(170, 160, 142),
                    Justify::Right,
                );
                spawn_battle_text(
                    mp,
                    ability.mp_cost.to_string(),
                    font,
                    20.0,
                    if affordable {
                        battle_teal()
                    } else {
                        battle_ember()
                    },
                    Justify::Right,
                );
            });
        });
}

fn spawn_item_row(
    parent: &mut ChildSpawnerCommands<'_>,
    item: &BattleItemChoice,
    index: usize,
    selected: bool,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            row_node(),
            BackgroundColor(if selected {
                battle_row_active()
            } else {
                battle_row()
            }),
            BorderColor::all(if selected {
                battle_border_active()
            } else {
                battle_row_border()
            }),
            PickerRow(index),
        ))
        .with_children(|row| {
            row.spawn((
                crate::item_images::ItemIcon::new(&item.id),
                Node {
                    width: px(36),
                    height: px(36),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            spawn_battle_text(row, &item.name, font, 16.0, battle_ink(), Justify::Left).insert(
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            );
            spawn_battle_text(
                row,
                format!("×{}", item.quantity),
                font,
                16.0,
                battle_ink(),
                Justify::Right,
            );
        });
}

#[derive(SystemParam)]
pub(super) struct DeckPointer<'w, 's> {
    pointer: Option<Res<'w, crate::gameplay_canvas::pointer::CanvasPointer>>,
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
    wheel: MessageReader<'w, 's, MouseWheel>,
    groups: Query<
        'w,
        's,
        (
            &'static GroupButton,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    rows: Query<
        'w,
        's,
        (
            &'static PickerRow,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    commands: Query<
        'w,
        's,
        (
            &'static BattleCommandRow,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    pickers: Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform), With<Picker>>,
    arrows: Query<
        'w,
        's,
        (
            &'static ScrollIndicator,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
}

pub(super) fn handle_pointer(
    mut input: DeckPointer,
    state: Option<ResMut<BattleState>>,
    deck: Option<ResMut<BattleDeck>>,
) {
    let (Some(mut state), Some(mut deck)) = (state, deck) else {
        input.wheel.clear();
        return;
    };
    if input
        .keys
        .as_ref()
        .is_some_and(|keys| keys.just_pressed(KeyCode::KeyL))
    {
        deck.log_position = (deck.log_position + 1) % 3;
    }
    let wheel: f32 = input
        .wheel
        .read()
        .map(|event| {
            event.y
                * if event.unit == MouseScrollUnit::Line {
                    1.0
                } else {
                    1.0 / 40.0
                }
        })
        .sum();
    let Some(pointer) = input.pointer.as_deref() else {
        return;
    };
    if wheel != 0.0
        && input
            .pickers
            .iter()
            .any(|(node, transform)| pointer.is_over(node, transform))
    {
        let delta = if wheel < 0.0 { 1 } else { -1 };
        if state.phase == BattlePhase::Ability {
            deck.list_focused = true;
            deck.move_spell(&mut state, delta);
        } else if state.phase == BattlePhase::Item {
            state.item_index = wrap_index(state.item_index, delta, state.item_choices.len());
        }
    }
    if !pointer.just_clicked() {
        return;
    }
    if matches!(state.phase, BattlePhase::Ability | BattlePhase::Item) {
        for (arrow, node, transform) in &input.arrows {
            if !pointer.is_over(node, transform) {
                continue;
            }
            let delta = match arrow {
                ScrollIndicator::Up => -1,
                ScrollIndicator::Down => 1,
                ScrollIndicator::Thumb => continue,
            };
            if state.phase == BattlePhase::Ability {
                deck.list_focused = true;
                deck.move_spell(&mut state, delta);
            } else if !state.item_choices.is_empty() {
                state.item_index = wrap_index(state.item_index, delta, state.item_choices.len());
            }
            return;
        }
    }
    if matches!(
        state.phase,
        BattlePhase::Command | BattlePhase::Ability | BattlePhase::Item
    ) {
        for (command, node, transform) in &input.commands {
            if pointer.is_over(node, transform) {
                state.command_index = command.0;
                state.phase = BattlePhase::Command;
                deck.pointer_confirm = true;
                return;
            }
        }
    }
    if state.phase == BattlePhase::Ability {
        for (button, node, transform) in &input.groups {
            if pointer.is_over(node, transform) && BattleDeck::enabled(&state, button.group) {
                deck.group = button.group;
                deck.list_focused = false;
                deck.scroll = ScrollMemory::default();
                deck.select_first(&mut state);
                return;
            }
        }
    }
    if matches!(state.phase, BattlePhase::Ability | BattlePhase::Item) {
        for (row, node, transform) in &input.rows {
            if !pointer.is_over(node, transform) {
                continue;
            }
            if state.phase == BattlePhase::Ability {
                if let Some(choice) = deck.choices(&state).get(row.0) {
                    state.ability_index = state
                        .battle_ability_indices()
                        .iter()
                        .position(|index| index == choice)
                        .unwrap_or(0);
                    deck.list_focused = true;
                    deck.pointer_confirm = true;
                }
            } else {
                state.item_index = row.0;
                deck.pointer_confirm = true;
            }
            return;
        }
    }
}

fn progress(member: &crate::runtime_member::RuntimeMember, catalog: &FieldMenuCatalog) -> f32 {
    if member.experience_next() == 0 {
        return 100.0;
    }
    let floor = if member.level() <= 1 {
        0
    } else {
        catalog.class(member.class_id()).map_or(0, |class| {
            crate::runtime_member::experience_required(class, member.level())
        })
    };
    meter_percent(
        member.experience().saturating_sub(floor),
        member.experience_next().saturating_sub(floor),
    )
}

pub(super) fn spawn_experience(
    parent: &mut ChildSpawnerCommands<'_>,
    index: usize,
    id: &str,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    font: &Handle<Font>,
) {
    let fraction = game
        .party()
        .member(id)
        .map_or(0.0, |member| progress(member, catalog));
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(14),
                flex_shrink: 0.0,
                position_type: PositionType::Relative,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(48, 43, 35)),
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    width: percent(fraction),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(167, 133, 69)),
                ExperienceFill(index),
            ));
            spawn_battle_text(
                bar,
                format!("EXP {fraction:.0}%"),
                font,
                10.0,
                battle_ink(),
                Justify::Center,
            )
            .insert(ExperienceText(index));
        });
}

pub(super) fn sync_progress(
    game: Option<Res<GameState>>,
    catalog: Res<FieldMenuCatalog>,
    state: Option<Res<BattleState>>,
    mut levels: Query<(&PartyLevel, &mut Text), Without<ExperienceText>>,
    mut fills: Query<(&ExperienceFill, &mut Node)>,
    mut labels: Query<(&ExperienceText, &mut Text), Without<PartyLevel>>,
) {
    let (Some(game), Some(state)) = (game, state) else {
        return;
    };
    if !game.is_changed() && !state.is_added() {
        return;
    }
    let member_at = |index| {
        state
            .actor(CombatantKey::party(index))
            .and_then(|actor| game.party().member(&actor.id))
    };
    for (marker, mut text) in &mut levels {
        if let Some(member) = member_at(marker.0) {
            text.0 = format!("Lv {}", member.level());
        }
    }
    for (marker, mut node) in &mut fills {
        if let Some(member) = member_at(marker.0) {
            node.width = percent(progress(member, &catalog));
        }
    }
    for (marker, mut text) in &mut labels {
        if let Some(member) = member_at(marker.0) {
            text.0 = if member.experience_next() == 0 {
                "MAX LEVEL".to_owned()
            } else {
                format!("EXP {:.0}%", progress(member, &catalog))
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::tests::{actor, state_with};

    fn ability(id: &str) -> Ability {
        let catalog = FieldMenuCatalog::production_class_fixture();
        ["cleric", "hero", "rogue", "sorcerer", "warrior"]
            .into_iter()
            .find_map(|class| {
                catalog
                    .class(class)?
                    .abilities
                    .iter()
                    .find(|ability| ability.id == id)
                    .cloned()
            })
            .unwrap_or_else(|| panic!("missing authored ability {id}"))
    }

    fn state(ids: &[&str]) -> BattleState {
        let mut caster = actor(BattleSide::Party, 0, 100, 100);
        caster.abilities = ids.iter().map(|id| ability(id)).collect();
        state_with(vec![caster])
    }

    #[test]
    fn healing_and_recovery_take_priority_over_their_element_or_effect_kind() {
        for id in [
            "heal",
            "heal_all",
            "revive",
            "kindred_light",
            "cure",
            "regen",
        ] {
            assert_eq!(SpellGroup::of(&ability(id)), SpellGroup::Heal, "{id}");
        }
        assert_eq!(SpellGroup::of(&ability("barrier")), SpellGroup::Holy);
        assert_eq!(SpellGroup::of(&ability("taunt")), SpellGroup::Utility);
        assert_eq!(SpellGroup::of(&ability("fireball")), SpellGroup::Fire);
    }

    #[test]
    fn opening_spells_resets_to_first_enabled_group_and_keeps_original_ability_indices() {
        let mut state = state(&["teleport", "heal", "earth_spike", "fire_bolt", "fireball"]);
        let mut deck = BattleDeck {
            group: SpellGroup::Heal,
            list_focused: true,
            ..default()
        };
        deck.open(&mut state);
        assert_eq!(deck.group, SpellGroup::Fire);
        assert!(!deck.list_focused);
        assert_eq!(deck.choices(&state), vec![3, 4]);
        assert_eq!(state.battle_ability_indices()[state.ability_index], 3);
        deck.move_spell(&mut state, 1);
        assert_eq!(state.battle_ability_indices()[state.ability_index], 4);
        deck.move_group(&mut state, 1);
        deck.open(&mut state);
        assert_eq!(state.battle_ability_indices()[state.ability_index], 3);
    }

    #[test]
    fn group_navigation_skips_empty_groups_and_wraps_in_both_directions() {
        let mut state = state(&["heal", "earth_spike", "fire_bolt"]);
        let mut deck = BattleDeck::default();
        deck.open(&mut state);
        deck.move_group(&mut state, 1);
        assert_eq!(deck.group, SpellGroup::Earth);
        deck.move_group(&mut state, 1);
        assert_eq!(deck.group, SpellGroup::Heal);
        deck.move_group(&mut state, 1);
        assert_eq!(deck.group, SpellGroup::Fire);
        deck.move_group(&mut state, -1);
        assert_eq!(deck.group, SpellGroup::Heal);
        assert!(!BattleDeck::enabled(&state, SpellGroup::Utility));
    }

    #[test]
    fn an_empty_caster_has_no_selectable_groups_and_navigation_is_safe() {
        let mut state = state(&["teleport"]);
        let mut deck = BattleDeck::default();
        deck.open(&mut state);
        deck.move_group(&mut state, 1);
        deck.move_spell(&mut state, -1);
        assert!(deck.choices(&state).is_empty());
        assert!(
            SpellGroup::ALL
                .into_iter()
                .all(|group| !BattleDeck::enabled(&state, group))
        );
    }
}
