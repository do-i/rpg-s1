use bevy::ecs::system::SystemParam;
use bevy::{asset::LoadState, prelude::*};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    encounter::{BattleEntry, BattleSide, restore_pre_battle_context},
    field_menu_domain::FieldMenuCatalog,
    game_state::GameState,
    scenario_balance::BalanceData,
    scenario_battle_background::{BattleBackgroundCatalog, GroundRect},
    scenario_enemy::EnemySize,
    scenario_inventory::ScenarioInventory,
    scenario_path::{ScenarioRelativePath, ScenarioRelativePathError},
    scenario_root::ScenarioRoot,
    sfx_cue::{MenuSfx, PlaySfx, cue},
    tsx_atlas_asset::TsxAtlasAsset,
    world_encounter::WorldEncounterRestore,
};

use super::{
    ability::{AbilityError, AbilityTargetPlan, resolve_ability, target_plan},
    fx::AttackKind,
    item::{ItemUseError, battle_item, resolve_item, target_plan as item_target_plan},
    model::{
        BattleCommand, BattleItemChoice, BattlePhase, BattleState, CombatantKey, FleeOutcome,
        TargetGroup, TargetSelector,
    },
    rewards::apply_rewards,
    rules::{flee_chance, phase_after_flee_confirmation, roll_flee, wrap_index},
};

const LPC_COLUMNS: u32 = 9;
/// LPC row layout, shared by every authored `*_battle.tsx` sheet: row 2 is spellcast-facing-down
/// over 7 frames, row 6 is thrust-facing-down over 8 (`battle_enemy_area_renderer.py:44-49`). The
/// idle pose is the first frame of the spellcast row.
const SPELLCAST_ROW: u32 = 2;
const SPELLCAST_FRAMES: u32 = 7;
const THRUST_ROW: u32 = 6;
const THRUST_FRAMES: u32 = 8;
const BATTLE_IDLE_TILE: u32 = SPELLCAST_ROW * LPC_COLUMNS;
const ENEMY_AREA_HEIGHT: f32 = 468.0;
const ENEMY_CANVAS_WIDTH: f32 = 1280.0;
const ENEMY_GROUND_NUDGE: f32 = 10.0;
const ENEMY_BAR_RESERVE: f32 = 32.0;
const BREATH_TOP_FRACTION: f32 = 0.60;
const BREATH_MAX_SQUASH: f32 = 2.0;
const BREATH_PERIOD_SECONDS: f32 = 1.4;
const BREATH_PHASE_OFFSET: f32 = 0.6;
/// Gap between a status pill stack and the corner of the frame it sits in.
const BADGE_INSET: f32 = 5.0;
const PARTY_PORTRAIT_SIZE: f32 = 100.0;
const PARTY_CARD_WIDTH: f32 = 108.0;
const PARTY_CARD_HEIGHT: f32 = 202.0;
const PARTY_CARD_GAP: f32 = 10.0;
/// Every slot the party can ever fill, drawn whether or not it is occupied.
const PARTY_SLOT_COUNT: usize = 5;
const PANEL_PADDING: f32 = 16.0;
const COMMAND_ROW_BORDER: f32 = 1.0;
const COMMAND_ROW_PADDING_X: f32 = 12.0;
const PANEL_BORDER_ACTIVE: f32 = 2.0;
/// Chrome between a command row's text and the panel's outer edge.
const COMMAND_ROW_CHROME: f32 =
    2.0 * (PANEL_BORDER_ACTIVE + PANEL_PADDING + COMMAND_ROW_BORDER + COMMAND_ROW_PADDING_X);
/// Room reserved for the widest row label the panel ever shows.
///
/// The longest is an ability row, `"{name}  {mp} MP"` — "Fortress Stance  12 MP"
/// measures 177px in Philosopher-Regular at the 18px row size, so this leaves a
/// margin for longer authored names and for rasterizer differences.
const COMMAND_LABEL_RESERVE: f32 = 200.0;
/// Fixed so the panel stops resizing as the log message and row labels change.
const COMMAND_PANEL_WIDTH: f32 = COMMAND_LABEL_RESERVE + COMMAND_ROW_CHROME;
const COMMANDS: [BattleCommand; 4] = [
    BattleCommand::Attack,
    BattleCommand::Spell,
    BattleCommand::Item,
    BattleCommand::Run,
];

pub(crate) struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .add_systems(OnEnter(AppState::Battle), enter_battle)
            .add_systems(
                Update,
                (
                    drive_battle_assets,
                    position_enemy_cards,
                    animate_enemy_breathing,
                    handle_battle_input,
                    super::fx::route_battle_fx,
                    super::fx::animate_battle_fx,
                    super::fx::animate_battle_shake,
                    sync_party_cards,
                    sync_party_meters,
                    sync_status_badges,
                    sync_battle_commands,
                    sync_battle_message,
                    sync_enemy_cards,
                    sync_enemy_meters,
                    super::reward_modal::sync_reward_modal,
                )
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            )
            .add_systems(OnExit(AppState::Battle), cleanup_battle);
    }
}

#[derive(Component)]
pub(super) struct BattleUi;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnemySpritePart {
    Upper,
    Lower,
}

#[derive(Component)]
struct BattleEnemyImage {
    index: usize,
    part: EnemySpritePart,
    split_fraction: f32,
}

#[derive(Component)]
struct BattleEnemyUpperBody {
    index: usize,
    base_height: f32,
}

#[derive(Component)]
struct BattleEnemyLabel(usize);

#[derive(Component)]
struct BattleEnemyCard(usize);

#[derive(Component)]
pub(super) struct BattleEnemyFrame(pub(super) usize);

#[derive(Component)]
struct BattleEnemyHpFill(usize);

#[derive(Component)]
pub(super) struct BattlePartyCard(pub(super) usize);

/// Marks a reserved-but-unfilled party slot. Never carries a [`BattlePartyCard`],
/// which is what keeps every party sync system from touching it.
#[derive(Component)]
struct BattlePartyEmptySlot;

#[derive(Component)]
struct BattlePartyPortrait(usize);

#[derive(Component)]
struct BattlePartyName(usize);

#[derive(Clone, Copy, Component)]
struct BattlePartyMeter {
    index: usize,
    kind: BattleMeterKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BattleMeterKind {
    Health,
    Mana,
}

#[derive(Component)]
struct BattlePartyMeterFill(BattlePartyMeter);

#[derive(Component)]
struct BattlePartyMeterText(BattlePartyMeter);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BattlePanelKind {
    Party,
    Command,
    Log,
}

#[derive(Component)]
struct BattlePanelTitle(BattlePanelKind);

#[derive(Component)]
struct BattleCommandRow(usize);

#[derive(Component)]
struct BattleCommandLabel(usize);

/// One badge pill on a combatant's frame. Slots are pre-spawned and hidden, so a status landing
/// mid-battle never has to spawn UI from a sync system.
#[derive(Clone, Copy, Component)]
struct BattleStatusBadge {
    key: CombatantKey,
    slot: usize,
}

#[derive(Clone, Copy, Component)]
struct BattleStatusBadgeLabel {
    key: CombatantKey,
    slot: usize,
}

#[derive(Component)]
struct BattleMessageText;

#[derive(Component)]
struct BattleTargetText;

#[derive(Debug, Resource)]
pub(super) struct BattleAssetState {
    atlases: Vec<Option<Handle<TsxAtlasAsset>>>,
    backgrounds: Handle<BattleBackgroundCatalog>,
    pub(super) font: Handle<Font>,
}

#[cfg(test)]
impl BattleAssetState {
    /// An asset set with nothing loaded, for systems that only read the font handle.
    pub(super) fn test_stub() -> Self {
        Self {
            atlases: Vec::new(),
            backgrounds: Handle::default(),
            font: Handle::default(),
        }
    }
}

fn battle_enemy_atlas_path(
    sprite_id: &str,
) -> Result<ScenarioRelativePath, ScenarioRelativePathError> {
    ScenarioRelativePath::try_from(
        format!("assets/sprites/enemies/{sprite_id}_battle.tsx").as_str(),
    )
}

fn enter_battle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    catalog: Res<FieldMenuCatalog>,
    game: Res<GameState>,
    entry: Option<Res<BattleEntry>>,
) {
    let Some(entry) = entry else {
        return;
    };
    let mut state = BattleState::from_entry(&entry);
    state.item_choices = catalog
        .ordered_items()
        .filter_map(battle_item)
        .filter_map(|item| {
            let quantity = game.repository().item_count(&item.id);
            (quantity > 0).then(|| BattleItemChoice {
                id: item.id.clone(),
                name: item.name.clone(),
                quantity,
            })
        })
        .collect();
    let atlases = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .map(|participant| {
            battle_enemy_atlas_path(&participant.sprite_id)
                .ok()
                .map(|path| asset_server.load(root.resolve(&path)))
        })
        .collect::<Vec<_>>();
    let background = ScenarioRelativePath::try_from(entry.background_asset.as_str())
        .ok()
        .map(|path| asset_server.load(root.resolve(&path)))
        .unwrap_or_default();
    let (Some(backgrounds_path), Some(font_path)) = (
        inventory.battle_backgrounds.as_ref(),
        inventory.font.as_ref(),
    ) else {
        return;
    };
    let backgrounds = asset_server.load(root.resolve(backgrounds_path));
    let font = asset_server.load(root.resolve(font_path));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(battle_floor()),
            GlobalZIndex(200),
            Pickable::IGNORE,
            BattleUi,
        ))
        .with_children(|root_node| {
            spawn_enemy_area(root_node, &entry, background, &font);
            spawn_battle_panels(root_node, &entry, &asset_server, &root, &font);
        });
    commands.insert_resource(BattleAssetState {
        atlases,
        backgrounds,
        font,
    });
    commands.insert_resource(super::fx::BattleFxRouter::default());
    commands.insert_resource(super::fx::BattleAttackAnimations::default());
    commands.insert_resource(state);
}

fn spawn_enemy_area(
    parent: &mut ChildSpawnerCommands<'_>,
    entry: &BattleEntry,
    background: Handle<Image>,
    font: &Handle<Font>,
) {
    let enemies = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .collect::<Vec<_>>();
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(ENEMY_AREA_HEIGHT),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            ImageNode::new(background).with_mode(NodeImageMode::Stretch),
        ))
        .with_children(|area| {
            area.spawn(Node {
                width: px(ENEMY_CANVAS_WIDTH),
                height: px(ENEMY_AREA_HEIGHT),
                position_type: PositionType::Relative,
                ..default()
            })
            .with_children(|battlefield| {
                for (index, participant) in enemies.iter().enumerate() {
                    let sprite_size = enemy_sprite_size(participant);
                    let card_width = sprite_size.max(80.0);
                    let (left, top) = enemy_card_position(
                        full_enemy_ground(),
                        sprite_size,
                        card_width,
                        enemies.len(),
                        index,
                    );
                    battlefield
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(left),
                                top: px(top),
                                width: px(card_width),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BattleEnemyCard(index),
                        ))
                        .with_children(|card| {
                            card.spawn((
                                Node {
                                    width: px(sprite_size + 4.0),
                                    height: px(sprite_size + 4.0),
                                    border: UiRect::all(px(2)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border_radius: BorderRadius::all(px(5)),
                                    ..default()
                                },
                                BorderColor::all(Color::NONE),
                                BattleEnemyFrame(index),
                            ))
                            .with_children(|frame| {
                                let top_height = (sprite_size * BREATH_TOP_FRACTION).floor();
                                let split_fraction = top_height / sprite_size;
                                frame
                                    .spawn(Node {
                                        position_type: PositionType::Relative,
                                        width: px(sprite_size),
                                        height: px(sprite_size),
                                        ..default()
                                    })
                                    .with_children(|sprite| {
                                        sprite.spawn((
                                            ImageNode::solid_color(Color::srgba_u8(
                                                42, 58, 90, 210,
                                            )),
                                            Node {
                                                position_type: PositionType::Absolute,
                                                top: px(0),
                                                width: percent(100),
                                                height: px(top_height),
                                                ..default()
                                            },
                                            BattleEnemyImage {
                                                index,
                                                part: EnemySpritePart::Upper,
                                                split_fraction,
                                            },
                                            BattleEnemyUpperBody {
                                                index,
                                                base_height: top_height,
                                            },
                                        ));
                                        sprite.spawn((
                                            ImageNode::solid_color(Color::srgba_u8(
                                                42, 58, 90, 210,
                                            )),
                                            Node {
                                                position_type: PositionType::Absolute,
                                                top: px(top_height),
                                                width: percent(100),
                                                height: px(sprite_size - top_height),
                                                ..default()
                                            },
                                            BattleEnemyImage {
                                                index,
                                                part: EnemySpritePart::Lower,
                                                split_fraction,
                                            },
                                        ));
                                    });
                                spawn_status_badges(frame, CombatantKey::enemy(index), font);
                            });
                            card.spawn((
                                Node {
                                    position_type: PositionType::Relative,
                                    width: percent(100),
                                    height: px(22),
                                    margin: UiRect::top(px(4)),
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(4)),
                                    overflow: Overflow::clip(),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgb_u8(42, 42, 42)),
                                BorderColor::all(Color::srgb_u8(205, 198, 178)),
                            ))
                            .with_children(|bar| {
                                bar.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: px(0),
                                        width: percent(100),
                                        height: percent(100),
                                        ..default()
                                    },
                                    BackgroundColor(enemy_health()),
                                    BattleEnemyHpFill(index),
                                ));
                                spawn_battle_text(
                                    bar,
                                    participant.name.clone(),
                                    font,
                                    12.0,
                                    Color::WHITE,
                                    Justify::Center,
                                )
                                .insert(BattleEnemyLabel(index));
                            });
                        });
                }
            });
        });
}

fn spawn_battle_panels(
    parent: &mut ChildSpawnerCommands<'_>,
    entry: &BattleEntry,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    font: &Handle<Font>,
) {
    let party = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Party)
        .collect::<Vec<_>>();
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(260),
            padding: UiRect::all(px(8)),
            column_gap: px(8),
            ..default()
        })
        .with_children(|panels| {
            spawn_battle_panel(
                panels,
                Node {
                    width: px(party_panel_width()),
                    height: percent(100),
                    flex_shrink: 0.0,
                    ..default()
                },
                "Party",
                BattlePanelKind::Party,
                false,
                font,
                |panel| spawn_party_cards(panel, &party, asset_server, root, font),
            );
            panels
                .spawn(Node {
                    height: percent(100),
                    flex_grow: 1.0,
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|right| {
                    spawn_battle_panel(
                        right,
                        Node {
                            width: px(COMMAND_PANEL_WIDTH),
                            height: percent(100),
                            flex_grow: 0.0,
                            flex_shrink: 0.0,
                            ..default()
                        },
                        "Command",
                        BattlePanelKind::Command,
                        true,
                        font,
                        |panel| spawn_command_rows(panel, font),
                    );
                    spawn_battle_panel(
                        right,
                        Node {
                            height: percent(100),
                            flex_grow: 1.0,
                            min_width: px(0),
                            ..default()
                        },
                        "Log",
                        BattlePanelKind::Log,
                        false,
                        font,
                        |panel| spawn_message_log(panel, font),
                    );
                });
        });
}

fn spawn_battle_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    mut node: Node,
    title: &str,
    kind: BattlePanelKind,
    active: bool,
    font: &Handle<Font>,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    node.flex_direction = FlexDirection::Column;
    node.padding = UiRect::all(px(PANEL_PADDING));
    node.row_gap = px(8);
    node.border = UiRect::all(px(if active { PANEL_BORDER_ACTIVE } else { 1.0 }));
    node.border_radius = BorderRadius::all(px(6));
    parent
        .spawn((
            node,
            BackgroundColor(battle_panel()),
            BorderColor::all(if active {
                battle_border_active()
            } else {
                battle_border()
            }),
        ))
        .with_children(|panel| {
            spawn_battle_text(panel, title, font, 18.0, battle_gold(), Justify::Left)
                .insert(BattlePanelTitle(kind));
            panel.spawn((
                Node {
                    width: percent(100),
                    height: px(1),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(126, 98, 55, 150)),
            ));
            content(panel);
        });
}

fn spawn_party_cards(
    parent: &mut ChildSpawnerCommands<'_>,
    party: &[&crate::encounter::BattleParticipant],
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    font: &Handle<Font>,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            column_gap: px(PARTY_CARD_GAP),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            // The panel always shows every slot the party can ever fill, so the
            // layout stops reflowing as companions are recruited.
            for index in 0..PARTY_SLOT_COUNT {
                match party.get(index) {
                    Some(participant) => {
                        spawn_party_card(row, index, participant, asset_server, root, font);
                    }
                    None => spawn_empty_party_slot(row, font),
                }
            }
        });
}

fn party_card_node(filled: bool) -> Node {
    Node {
        width: px(PARTY_CARD_WIDTH),
        height: px(PARTY_CARD_HEIGHT),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        padding: UiRect::all(px(4)),
        row_gap: px(4),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(5)),
        justify_content: if filled {
            JustifyContent::FlexStart
        } else {
            JustifyContent::Center
        },
        ..default()
    }
}

fn spawn_party_card(
    row: &mut ChildSpawnerCommands<'_>,
    index: usize,
    participant: &crate::encounter::BattleParticipant,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    font: &Handle<Font>,
) {
    let portrait = ScenarioRelativePath::try_from(
        format!("assets/images/{}_profile.png", participant.id).as_str(),
    )
    .ok()
    .map(|path| asset_server.load(root.resolve(&path)))
    .unwrap_or_default();
    row.spawn((
        party_card_node(true),
        BackgroundColor(battle_row()),
        BorderColor::all(battle_row_border()),
        BattlePartyCard(index),
    ))
    .with_children(|card| {
        card.spawn((
            ImageNode::new(portrait).with_mode(NodeImageMode::Stretch),
            Node {
                width: px(PARTY_PORTRAIT_SIZE),
                height: px(PARTY_PORTRAIT_SIZE),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(24, 24, 31)),
            BattlePartyPortrait(index),
        ));
        spawn_battle_text(
            card,
            participant.name.clone(),
            font,
            17.0,
            battle_ink(),
            Justify::Center,
        )
        .insert(BattlePartyName(index));
        spawn_party_meter(
            card,
            index,
            BattleMeterKind::Health,
            participant.health,
            participant.max_health,
            font,
        );
        if participant.max_mana > 0 {
            spawn_party_meter(
                card,
                index,
                BattleMeterKind::Mana,
                participant.mana,
                participant.max_mana,
                font,
            );
        }
        spawn_status_badges(card, CombatantKey::party(index), font);
    });
}

/// A slot no companion has filled yet: the same cell outline holding a dimmed
/// avatar placeholder. It carries no [`BattlePartyCard`] marker, so every sync
/// system ignores it for as long as it stays empty.
fn spawn_empty_party_slot(row: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    row.spawn((
        party_card_node(false),
        BackgroundColor(empty_slot_fill()),
        BorderColor::all(empty_slot_border()),
        BattlePartyEmptySlot,
    ))
    .with_children(|card| {
        card.spawn((
            Node {
                width: px(PARTY_PORTRAIT_SIZE),
                height: px(PARTY_PORTRAIT_SIZE),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(24, 24, 31)),
        ))
        .with_children(|frame| {
            // Head and shoulders, the way a contact avatar reads as "nobody yet".
            frame.spawn((
                Node {
                    width: px(30),
                    height: px(30),
                    margin: UiRect::bottom(px(6)),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(percent(50)),
                    ..default()
                },
                BackgroundColor(empty_slot_ink()),
            ));
            frame.spawn((
                Node {
                    width: px(58),
                    height: px(34),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::top(percent(50)),
                    ..default()
                },
                BackgroundColor(empty_slot_ink()),
            ));
        });
        spawn_battle_text(card, "Empty", font, 15.0, empty_slot_ink(), Justify::Center);
    });
}

fn spawn_party_meter(
    parent: &mut ChildSpawnerCommands<'_>,
    index: usize,
    kind: BattleMeterKind,
    value: u32,
    maximum: u32,
    font: &Handle<Font>,
) {
    let marker = BattlePartyMeter { index, kind };
    parent
        .spawn((
            Node {
                position_type: PositionType::Relative,
                width: percent(100),
                height: px(18),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(17, 17, 22)),
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    width: percent(meter_percent(value, maximum)),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(meter_color(kind, value, maximum)),
                BattlePartyMeterFill(marker),
            ));
            spawn_battle_text(
                bar,
                meter_label(kind, value, maximum),
                font,
                12.0,
                battle_ink(),
                Justify::Center,
            )
            .insert(BattlePartyMeterText(marker));
        });
}

/// Stacks the combatant's status pills into the top-right corner of its frame.
///
/// Every slot is spawned up front and hidden; [`sync_status_badges`] only ever toggles and
/// re-colors them. The source pins its single badge to the same corner
/// (`battle_party_panel_renderer.py:141-151`).
fn spawn_status_badges(
    parent: &mut ChildSpawnerCommands<'_>,
    key: CombatantKey,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(BADGE_INSET),
                right: px(BADGE_INSET),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: px(2),
                ..default()
            },
            GlobalZIndex(15),
            Pickable::IGNORE,
        ))
        .with_children(|stack| {
            for slot in 0..super::badge::MAX_BADGES {
                stack
                    .spawn((
                        Node {
                            display: Display::None,
                            padding: UiRect::axes(px(4), px(1)),
                            border_radius: BorderRadius::all(px(3)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BattleStatusBadge { key, slot },
                    ))
                    .with_children(|pill| {
                        spawn_battle_text(pill, "", font, 11.0, battle_ink(), Justify::Center)
                            .insert(BattleStatusBadgeLabel { key, slot });
                    });
            }
        });
}

/// Shows every combatant's active statuses as persistent pills.
///
/// Runs off the same `is_changed` gate as the other sync systems: statuses only move when the
/// resolver writes them, so there is nothing to recompute on a quiet frame.
fn sync_status_badges(
    state: Option<Res<BattleState>>,
    mut pills: Query<(&BattleStatusBadge, &mut Node, &mut BackgroundColor)>,
    mut labels: Query<(&BattleStatusBadgeLabel, &mut Text, &mut TextColor)>,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    let badges_for = |key: CombatantKey| {
        state
            .actor(key)
            .filter(|actor| actor.is_alive())
            .map(|actor| super::badge::badges(&actor.status_effects))
            .unwrap_or_default()
    };
    for (marker, mut node, mut background) in &mut pills {
        let badge = badges_for(marker.key).get(marker.slot).copied();
        node.display = if badge.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        background.0 = badge.map_or(Color::NONE, |badge| badge.fill);
    }
    for (marker, mut text, mut color) in &mut labels {
        let badge = badges_for(marker.key).get(marker.slot).copied();
        text.0 = badge
            .map(|badge| badge.label.to_owned())
            .unwrap_or_default();
        color.0 = badge.map_or(Color::NONE, |badge| badge.ink);
    }
}

fn spawn_command_rows(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|list| {
            for (index, command) in COMMANDS.iter().enumerate() {
                list.spawn((
                    Node {
                        width: percent(100),
                        min_height: px(36),
                        flex_grow: 1.0,
                        padding: UiRect::axes(px(COMMAND_ROW_PADDING_X), px(6)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(5)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(battle_row()),
                    BorderColor::all(battle_row_border()),
                    BattleCommandRow(index),
                ))
                .with_children(|row| {
                    spawn_battle_text(
                        row,
                        command.label(),
                        font,
                        18.0,
                        battle_ink(),
                        Justify::Left,
                    )
                    .insert(BattleCommandLabel(index));
                });
            }
        });
}

fn spawn_message_log(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|log| {
            spawn_battle_text(log, "", font, 18.0, battle_teal(), Justify::Left)
                .insert(BattleMessageText);
            spawn_battle_text(log, "", font, 15.0, battle_violet(), Justify::Left)
                .insert(BattleTargetText);
        });
}

pub(super) fn spawn_battle_text<'a>(
    parent: &'a mut ChildSpawnerCommands<'_>,
    text: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    color: Color,
    justify: Justify,
) -> EntityCommands<'a> {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextLayout::new(justify, LineBreak::WordOrCharacter),
    ))
}

/// Fixed to [`PARTY_SLOT_COUNT`], so recruiting a companion never reflows the panel.
fn party_panel_width() -> f32 {
    let count = PARTY_SLOT_COUNT as f32;
    count * PARTY_CARD_WIDTH + (count - 1.0) * PARTY_CARD_GAP + PANEL_PADDING * 2.0
}

fn enemy_sprite_size(participant: &crate::encounter::BattleParticipant) -> f32 {
    let base = if participant.boss {
        96.0
    } else {
        match participant.enemy_size.unwrap_or(EnemySize::Medium) {
            EnemySize::Small => 52.0,
            EnemySize::Medium => 64.0,
            EnemySize::Large => 80.0,
            EnemySize::Boss => 96.0,
        }
    };
    (base * participant.sprite_scale_percent as f32 / 100.0).floor()
}

fn enemy_layout_offset(count: usize, index: usize) -> (f32, f32) {
    const ONE: [(f32, f32); 1] = [(0.0, 0.0)];
    const TWO: [(f32, f32); 2] = [(-80.0, 0.0), (80.0, 0.0)];
    const THREE: [(f32, f32); 3] = [(-110.0, -30.0), (0.0, 20.0), (110.0, -20.0)];
    const FOUR: [(f32, f32); 4] = [(-140.0, -20.0), (-45.0, 20.0), (45.0, -20.0), (140.0, 20.0)];
    const FIVE: [(f32, f32); 5] = [
        (-160.0, -30.0),
        (-80.0, 20.0),
        (0.0, -10.0),
        (80.0, 20.0),
        (160.0, -30.0),
    ];
    let offsets = match count {
        1 => &ONE[..],
        2 => &TWO[..],
        3 => &THREE[..],
        4 => &FOUR[..],
        _ => &FIVE[..],
    };
    offsets.get(index).copied().unwrap_or_default()
}

fn full_enemy_ground() -> GroundRect {
    GroundRect {
        x: 0,
        y: 0,
        width: ENEMY_CANVAS_WIDTH as i32,
        height: ENEMY_AREA_HEIGHT as i32,
    }
}

fn enemy_card_position(
    ground: GroundRect,
    sprite_size: f32,
    card_width: f32,
    enemy_count: usize,
    index: usize,
) -> (f32, f32) {
    let ground = if ground.width > 0 && ground.height > 0 {
        ground
    } else {
        full_enemy_ground()
    };
    let (offset_x, offset_y) = enemy_layout_offset(enemy_count, index);
    let ground_left = ground.x as f32;
    let ground_top = ground.y as f32;
    let ground_right = ground_left + ground.width as f32;
    let ground_bottom = ground_top + ground.height as f32;

    let minimum_x = ground_left + sprite_size / 2.0;
    let maximum_x = (ground_right - sprite_size / 2.0).max(minimum_x);
    let center_x = (ENEMY_CANVAS_WIDTH / 2.0 + offset_x).clamp(minimum_x, maximum_x);

    let center_y = ground_top + ground.height as f32 / 2.0 + ENEMY_GROUND_NUDGE + offset_y;
    let minimum_feet = ground_top;
    let maximum_feet = (ground_bottom - ENEMY_BAR_RESERVE).max(minimum_feet);
    let feet = (center_y + sprite_size / 2.0).clamp(minimum_feet, maximum_feet);

    (center_x - card_width / 2.0, feet - sprite_size - 2.0)
}

fn breath_squash(elapsed_seconds: f32, index: usize) -> f32 {
    let phase = elapsed_seconds * std::f32::consts::TAU / BREATH_PERIOD_SECONDS
        + index as f32 * BREATH_PHASE_OFFSET;
    (BREATH_MAX_SQUASH * (0.5 - 0.5 * phase.cos())).round()
}

fn meter_percent(value: u32, maximum: u32) -> f32 {
    if maximum == 0 {
        0.0
    } else {
        (value as f32 / maximum as f32 * 100.0).clamp(0.0, 100.0)
    }
}

fn meter_label(kind: BattleMeterKind, value: u32, maximum: u32) -> String {
    let label = match kind {
        BattleMeterKind::Health => "HP",
        BattleMeterKind::Mana => "MP",
    };
    format!("{label} {value}/{maximum}")
}

fn meter_color(kind: BattleMeterKind, value: u32, maximum: u32) -> Color {
    match kind {
        BattleMeterKind::Health if meter_percent(value, maximum) <= 25.0 => battle_ember(),
        BattleMeterKind::Health => battle_health(),
        BattleMeterKind::Mana => battle_mana(),
    }
}

pub(super) fn battle_ink() -> Color {
    Color::srgb_u8(242, 236, 211)
}

pub(super) fn battle_dim() -> Color {
    Color::srgb_u8(101, 96, 88)
}

pub(super) fn battle_gold() -> Color {
    Color::srgb_u8(231, 184, 86)
}

fn battle_ember() -> Color {
    Color::srgb_u8(203, 82, 47)
}

pub(super) fn battle_teal() -> Color {
    Color::srgb_u8(67, 166, 160)
}

pub(super) fn battle_violet() -> Color {
    Color::srgb_u8(126, 101, 204)
}

pub(super) fn battle_panel() -> Color {
    Color::srgba_u8(22, 22, 28, 228)
}

fn battle_floor() -> Color {
    Color::srgb_u8(17, 17, 40)
}

fn battle_border() -> Color {
    Color::srgb_u8(126, 98, 55)
}

pub(super) fn battle_border_active() -> Color {
    Color::srgb_u8(235, 190, 89)
}

pub(super) fn battle_row() -> Color {
    Color::srgba_u8(30, 30, 38, 210)
}

fn battle_row_active() -> Color {
    Color::srgba_u8(79, 51, 38, 230)
}

/// The targeting reticle, `C_TARGET` in the source (`battle_party_panel_renderer.py:25`).
fn battle_target() -> Color {
    Color::srgb_u8(204, 170, 255)
}

pub(super) fn battle_row_border() -> Color {
    Color::srgb_u8(82, 70, 50)
}

fn empty_slot_fill() -> Color {
    Color::srgba_u8(20, 20, 26, 150)
}

fn empty_slot_border() -> Color {
    Color::srgb_u8(54, 47, 36)
}

fn empty_slot_ink() -> Color {
    Color::srgb_u8(58, 56, 62)
}

fn battle_health() -> Color {
    Color::srgb_u8(52, 104, 82)
}

fn enemy_health() -> Color {
    Color::srgb_u8(68, 170, 68)
}

fn battle_mana() -> Color {
    Color::srgb_u8(88, 72, 138)
}

/// The sheet tile an enemy shows this frame.
///
/// An idle enemy holds the resting pose; a mid-action one steps through its row, holding the last
/// frame at the end rather than wrapping (`battle_enemy_area_renderer.py:200-207`).
fn enemy_sprite_tile(animation: Option<(AttackKind, f32)>) -> u32 {
    let Some((kind, progress)) = animation else {
        return BATTLE_IDLE_TILE;
    };
    let (row, frames) = match kind {
        AttackKind::Thrust => (THRUST_ROW, THRUST_FRAMES),
        AttackKind::Spellcast => (SPELLCAST_ROW, SPELLCAST_FRAMES),
    };
    let frame = ((progress * frames as f32) as u32).min(frames - 1);
    row * LPC_COLUMNS + frame
}

fn drive_battle_assets(
    asset_server: Res<AssetServer>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    assets: Option<Res<BattleAssetState>>,
    attacks: Option<Res<super::fx::BattleAttackAnimations>>,
    mut images: Query<(&BattleEnemyImage, &mut ImageNode)>,
) {
    let Some(assets) = assets else { return };
    for (marker, mut image) in &mut images {
        let Some(Some(handle)) = assets.atlases.get(marker.index) else {
            continue;
        };
        if matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)) {
            continue;
        }
        let Some(atlas) = atlases.get(handle) else {
            continue;
        };
        let animation = attacks
            .as_deref()
            .and_then(|attacks| attacks.progress(marker.index));
        if let Ok(sprite) = atlas.sprite_for_tile(enemy_sprite_tile(animation)) {
            image.image = sprite.image;
            image.texture_atlas = sprite.texture_atlas;
            let source_width = atlas.metadata().tile_width() as f32;
            let source_height = atlas.metadata().tile_height() as f32;
            let split = source_height * marker.split_fraction;
            image.rect = Some(match marker.part {
                EnemySpritePart::Upper => Rect::new(0.0, 0.0, source_width, split),
                EnemySpritePart::Lower => Rect::new(0.0, split, source_width, source_height),
            });
            image.image_mode = NodeImageMode::Stretch;
            image.color = Color::WHITE;
        }
    }
}

fn position_enemy_cards(
    catalogs: Res<Assets<BattleBackgroundCatalog>>,
    assets: Option<Res<BattleAssetState>>,
    entry: Option<Res<BattleEntry>>,
    mut cards: Query<(&BattleEnemyCard, &mut Node)>,
) {
    let (Some(assets), Some(entry)) = (assets, entry) else {
        return;
    };
    let ground = catalogs
        .get(&assets.backgrounds)
        .and_then(|catalog| {
            catalog
                .0
                .iter()
                .find(|background| background.id == entry.background_id)
        })
        .map_or_else(full_enemy_ground, |background| background.ground_rect);
    let enemies = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .collect::<Vec<_>>();
    for (marker, mut node) in &mut cards {
        let Some(participant) = enemies.get(marker.0) else {
            continue;
        };
        let sprite_size = enemy_sprite_size(participant);
        let (left, top) = enemy_card_position(
            ground,
            sprite_size,
            sprite_size.max(80.0),
            enemies.len(),
            marker.0,
        );
        node.left = px(left);
        node.top = px(top);
    }
}

/// Idle breathing, held flat while an enemy is mid-swing so the squash does not fight the attack
/// frames — the same suspension the source applies (`battle_enemy_area_renderer.py:120-124`).
fn animate_enemy_breathing(
    time: Res<Time>,
    attacks: Option<Res<super::fx::BattleAttackAnimations>>,
    mut upper_bodies: Query<(&BattleEnemyUpperBody, &mut Node)>,
) {
    for (marker, mut node) in &mut upper_bodies {
        let attacking = attacks
            .as_deref()
            .is_some_and(|attacks| attacks.progress(marker.index).is_some());
        let squash = if attacking {
            0.0
        } else {
            breath_squash(time.elapsed_secs(), marker.index)
        };
        node.top = px(squash);
        node.height = px((marker.base_height - squash).max(1.0));
    }
}

#[derive(SystemParam)]
struct BattleInputContext<'w> {
    balances: Res<'w, Assets<BalanceData>>,
    catalog: Res<'w, FieldMenuCatalog>,
    game: Option<ResMut<'w, GameState>>,
    entry: Option<Res<'w, BattleEntry>>,
}

fn handle_battle_input(
    mut commands: Commands,
    actions: Res<ActionState>,
    mut context: BattleInputContext,
    state: Option<ResMut<BattleState>>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
    mut timings: Option<ResMut<crate::frame_timing::FrameTimings>>,
    mut menu_sfx: MenuSfx,
) {
    let _measurement = timings
        .as_deref_mut()
        .map(|timings| timings.measure("battle.input_and_resolution"));
    let Some(entry) = context.entry.as_deref() else {
        return;
    };
    let Some(mut game) = context.game.take() else {
        return;
    };
    let Some(mut state) = state else {
        return;
    };
    let balances: &Assets<BalanceData> = &context.balances;
    let catalog: &FieldMenuCatalog = &context.catalog;
    match state.phase {
        BattlePhase::Start => state.begin_active_turn(game.rng_mut()),
        BattlePhase::Advance => state.advance(game.rng_mut()),
        BattlePhase::Command => {
            if actions.just_pressed(AppAction::Back) {
                attempt_flee(&mut state, balances, &mut game, &mut menu_sfx);
                return;
            }
            if let Some(movement) = actions.menu_navigation() {
                state.command_index = wrap_index(state.command_index, movement, COMMANDS.len());
                menu_sfx.hover();
            }
            if confirm_pressed(&actions) {
                let command = COMMANDS[state.command_index];
                if !state.command_available(command) {
                    state.message = match command {
                        BattleCommand::Spell => "No usable abilities.".to_owned(),
                        BattleCommand::Item => "No battle items available.".to_owned(),
                        BattleCommand::Run => "Can't escape from a boss!".to_owned(),
                        BattleCommand::Attack => "That action is unavailable.".to_owned(),
                    };
                    menu_sfx.blocked();
                    return;
                }
                menu_sfx.confirm();
                match command {
                    BattleCommand::Attack => {
                        state.target =
                            TargetSelector::new(TargetGroup::Enemy, &state.combatants, false);
                        if state.target.is_some() {
                            debug_assert!(state.phase.allows(BattlePhase::Target));
                            state.phase = BattlePhase::Target;
                            state.message = "Choose a living enemy. ESC cancels.".to_owned();
                        } else {
                            state.assess_result();
                        }
                    }
                    BattleCommand::Run => {
                        attempt_flee(&mut state, balances, &mut game, &mut menu_sfx);
                    }
                    BattleCommand::Spell => {
                        state.ability_index = 0;
                        state.pending_ability = None;
                        state.phase = BattlePhase::Ability;
                        state.message = "Choose an ability. ESC cancels.".to_owned();
                    }
                    BattleCommand::Item => {
                        state.item_index = 0;
                        state.pending_item = None;
                        state.phase = BattlePhase::Item;
                        state.message = "Choose an item. ESC cancels.".to_owned();
                    }
                }
            }
        }
        BattlePhase::Ability => {
            let choices = state.battle_ability_indices();
            if cancel_pressed(&actions) {
                state.phase = BattlePhase::Command;
                state.message = "Action cancelled.".to_owned();
            } else {
                if let Some(movement) = actions.menu_navigation() {
                    state.ability_index = wrap_index(state.ability_index, movement, choices.len());
                }
                if confirm_pressed(&actions)
                    && let Some(&ability_index) = choices.get(state.ability_index)
                {
                    begin_ability_targeting(&mut state, ability_index, game.rng_mut());
                }
            }
        }
        BattlePhase::Item => {
            if cancel_pressed(&actions) {
                state.phase = BattlePhase::Command;
                state.message = "Action cancelled.".to_owned();
            } else {
                if let Some(movement) = actions.menu_navigation() {
                    state.item_index =
                        wrap_index(state.item_index, movement, state.item_choices.len());
                }
                if confirm_pressed(&actions)
                    && let Some(choice) = state.item_choices.get(state.item_index).cloned()
                {
                    begin_item_targeting(&mut state, &choice.id, catalog);
                }
            }
        }
        BattlePhase::Target => {
            if actions.just_pressed(AppAction::Back) {
                state.target = None;
                state.phase = if state.pending_ability.is_some() {
                    BattlePhase::Ability
                } else if state.pending_item.is_some() {
                    BattlePhase::Item
                } else {
                    BattlePhase::Command
                };
                state.message = "Action cancelled.".to_owned();
            } else {
                // Enemies stand in a row, so Left/Right cycle the same pool Up/Down do
                // (`battle_input.py:106-109`).
                if let Some(movement) = target_navigation(&actions)
                    && let Some(target) = state.target.as_mut()
                {
                    target.navigate(movement);
                }
                if actions.just_pressed(AppAction::Confirm) {
                    let attacker = state.active_key();
                    let target = state.target.as_ref().map(TargetSelector::selected);
                    state.target = None;
                    if let (Some(attacker), Some(target)) = (attacker, target) {
                        if let Some(ability) = state.pending_ability.take() {
                            resolve_selected_ability(
                                &mut state,
                                attacker,
                                ability,
                                &[target],
                                game.rng_mut(),
                            );
                        } else if let Some(item_id) = state.pending_item.take() {
                            resolve_selected_item(
                                &mut state,
                                attacker,
                                &item_id,
                                target,
                                catalog,
                                game.repository_mut(),
                            );
                        } else {
                            state.resolve_physical(attacker, target, game.rng_mut());
                        }
                    }
                }
            }
        }
        BattlePhase::Resolve if actions.just_pressed(AppAction::Confirm) => state.assess_result(),
        BattlePhase::Victory if actions.just_pressed(AppAction::Confirm) => {
            let Some(balance) = balances.iter().next().map(|(_, value)| value) else {
                state.message = "Battle balance is still loading.".to_owned();
                return;
            };
            match apply_rewards(
                &mut state,
                &mut game,
                catalog,
                balance,
                entry.boss_completion_flag.as_deref(),
            ) {
                Ok(rewards) => {
                    debug_assert_eq!(state.rewards.as_ref(), Some(&rewards));
                    state.phase = BattlePhase::Rewards;
                    state.message = rewards.detail_message();
                }
                Err(error) => state.message = format!("Could not apply rewards: {error}"),
            }
        }
        BattlePhase::Rewards if actions.just_pressed(AppAction::Confirm) => {
            restore_world(&mut commands, &mut game, entry);
            transitions.write(AppStateTransitionRequest::new(AppState::World));
        }
        BattlePhase::Defeat => {
            transitions.write(AppStateTransitionRequest::new(AppState::GameOver));
        }
        BattlePhase::Flee if actions.just_pressed(AppAction::Confirm) => {
            let outcome = state.flee_outcome.unwrap_or(FleeOutcome::Failed);
            if phase_after_flee_confirmation(outcome).is_none() {
                restore_world(&mut commands, &mut game, entry);
                transitions.write(AppStateTransitionRequest::new(AppState::World));
            } else {
                state.phase = phase_after_flee_confirmation(outcome)
                    .expect("failed and blocked flee consume the active turn");
            }
        }
        _ => {}
    }
}

/// Confirm, or Right — the pinned engine accepts both on the command and sub-menus
/// (`battle_input.py:80,93`). Target selection is deliberately excluded: there Right steps the
/// pool instead.
fn confirm_pressed(actions: &ActionState) -> bool {
    actions.just_pressed(AppAction::Confirm) || actions.just_pressed(AppAction::Right)
}

/// Back, or Left — Left backs out of a sub-menu in the pinned engine (`battle_input.py:85`).
fn cancel_pressed(actions: &ActionState) -> bool {
    actions.just_pressed(AppAction::Back) || actions.just_pressed(AppAction::Left)
}

/// One step through the target pool: Left/Up step back, Right/Down step forward.
fn target_navigation(actions: &ActionState) -> Option<isize> {
    actions
        .menu_navigation()
        .or_else(|| actions.menu_navigation_horizontal())
}

fn begin_item_targeting(state: &mut BattleState, item_id: &str, catalog: &FieldMenuCatalog) {
    let Some(item) = catalog.item(item_id).and_then(battle_item) else {
        state.message = "That item cannot be used in battle.".to_owned();
        return;
    };
    let plan = item_target_plan(item);
    state.target = TargetSelector::new(plan.group, &state.combatants, plan.ko_eligible);
    if state.target.is_some() {
        state.pending_item = Some(item_id.to_owned());
        state.phase = BattlePhase::Target;
        state.message = "Choose a target. ESC cancels.".to_owned();
    } else {
        state.message = "No valid target.".to_owned();
    }
}

fn resolve_selected_item(
    state: &mut BattleState,
    source: CombatantKey,
    item_id: &str,
    target: CombatantKey,
    catalog: &FieldMenuCatalog,
    repository: &mut crate::runtime_repository::RuntimeRepository,
) {
    let Some(item) = catalog.item(item_id).and_then(battle_item) else {
        state.phase = BattlePhase::Item;
        state.message = "That item cannot be used in battle.".to_owned();
        return;
    };
    match resolve_item(state, source, item, target, repository) {
        Ok(_) => {
            if let Some(choice) = state
                .item_choices
                .iter_mut()
                .find(|choice| choice.id == item_id)
            {
                choice.quantity = choice.quantity.saturating_sub(1);
            }
            state.item_choices.retain(|choice| choice.quantity > 0);
            state.item_index = state
                .item_index
                .min(state.item_choices.len().saturating_sub(1));
        }
        Err(error) => {
            state.phase = BattlePhase::Item;
            state.message = match error {
                ItemUseError::InvalidTarget => "No valid target.".to_owned(),
                ItemUseError::Unavailable => "That item is no longer available.".to_owned(),
                ItemUseError::UnknownSource | ItemUseError::Unsupported => {
                    "That item cannot be used here.".to_owned()
                }
            };
        }
    }
}

fn begin_ability_targeting(
    state: &mut BattleState,
    ability_index: usize,
    rng: &mut crate::gameplay_rng::GameplayRng,
) {
    let Some(caster) = state.active_key() else {
        return;
    };
    let Some(ability) = state
        .actor(caster)
        .and_then(|actor| actor.abilities.get(ability_index))
        .cloned()
    else {
        return;
    };
    if state
        .actor(caster)
        .is_none_or(|actor| actor.mana < ability.mp_cost)
    {
        state.message = format!("Not enough MP for {}.", ability.name);
        return;
    }
    match target_plan(&ability) {
        AbilityTargetPlan::Select { group, ko_eligible } => {
            state.target = TargetSelector::new(group, &state.combatants, ko_eligible);
            if state.target.is_some() {
                state.pending_ability = Some(ability_index);
                state.phase = BattlePhase::Target;
                state.message = "Choose a target. ESC cancels.".to_owned();
            } else {
                state.message = "No valid target.".to_owned();
            }
        }
        AbilityTargetPlan::All { side, ko_eligible } => {
            let targets = state
                .combatants
                .iter()
                .filter(|actor| actor.key.side == side && actor.is_alive() != ko_eligible)
                .map(|actor| actor.key)
                .collect::<Vec<_>>();
            resolve_selected_ability(state, caster, ability_index, &targets, rng);
        }
        AbilityTargetPlan::SelfTarget => {
            resolve_selected_ability(state, caster, ability_index, &[caster], rng);
        }
    }
}

fn resolve_selected_ability(
    state: &mut BattleState,
    caster: CombatantKey,
    ability_index: usize,
    targets: &[CombatantKey],
    rng: &mut crate::gameplay_rng::GameplayRng,
) {
    if let Err(error) = resolve_ability(state, caster, ability_index, targets, rng) {
        state.phase = BattlePhase::Ability;
        state.message = match error {
            AbilityError::InsufficientMana => "Not enough MP.".to_owned(),
            AbilityError::Silenced => "Silence prevents spellcasting.".to_owned(),
            AbilityError::InvalidTarget => "No valid target.".to_owned(),
            AbilityError::UnknownCaster
            | AbilityError::UnknownAbility
            | AbilityError::Unsupported => "That ability cannot be used here.".to_owned(),
        };
    }
}

fn attempt_flee(
    state: &mut BattleState,
    balances: &Assets<BalanceData>,
    game: &mut GameState,
    menu_sfx: &mut MenuSfx,
) {
    let Some(balance) = balances.iter().next().map(|(_, value)| value) else {
        state.message = "Battle balance is still loading.".to_owned();
        menu_sfx.blocked();
        return;
    };
    let boss = state
        .combatants
        .iter()
        .any(|actor| actor.key.side == BattleSide::Enemy && actor.boss);
    let chance = flee_chance(state, balance);
    let outcome = roll_flee(boss, chance, game.rng_mut());
    state.flee_outcome = Some(outcome);
    state.phase = BattlePhase::Flee;
    state.message = match outcome {
        FleeOutcome::Success => "Got away safely! Press Enter.".to_owned(),
        FleeOutcome::Failed => "Couldn't escape! Press Enter.".to_owned(),
        FleeOutcome::Blocked => "Can't escape from a boss! Press Enter.".to_owned(),
    };
    // The pinned engine splits these: getting away plays the escape cue, a failed or boss-blocked
    // attempt plays the refusal (`battle_scene.py:246`).
    match outcome {
        FleeOutcome::Success => menu_sfx.play(cue::FLEE),
        FleeOutcome::Failed | FleeOutcome::Blocked => menu_sfx.blocked(),
    }
    state.transcript.push(format!("FLEE {outcome:?}"));
}

fn restore_world(commands: &mut Commands, game: &mut GameState, entry: &BattleEntry) {
    if restore_pre_battle_context(game, &entry.return_context).is_ok() {
        commands.insert_resource(WorldEncounterRestore {
            map_id: entry.return_context.map_id.clone(),
            enemies: entry.return_context.world_enemies.clone(),
        });
    }
}

fn sync_party_cards(
    state: Option<Res<BattleState>>,
    mut cards: Query<(&BattlePartyCard, &mut BackgroundColor, &mut BorderColor)>,
    mut portraits: Query<(&BattlePartyPortrait, &mut ImageNode)>,
    mut names: Query<(&BattlePartyName, &mut TextColor)>,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    let active = state.active_key();
    let selected_target = state.target.as_ref().map(TargetSelector::selected);
    for (marker, mut background, mut border) in &mut cards {
        let key = CombatantKey::party(marker.0);
        let Some(actor) = state.actor(key) else {
            continue;
        };
        let is_active = active == Some(key) && actor.is_alive();
        background.0 = if is_active {
            battle_row_active()
        } else {
            battle_row()
        };
        // A heal or a revive picks its target from these cards, so the reticle has to land on the
        // member the way it lands on an enemy sprite (`battle_party_panel_renderer.py:80-82`).
        border.set_all(if selected_target == Some(key) {
            battle_target()
        } else if is_active {
            battle_border_active()
        } else {
            battle_row_border()
        });
    }
    for (marker, mut portrait) in &mut portraits {
        let Some(actor) = state.actor(CombatantKey::party(marker.0)) else {
            continue;
        };
        portrait.color = if actor.is_alive() {
            Color::WHITE
        } else {
            Color::srgba(0.28, 0.28, 0.28, 0.72)
        };
    }
    for (marker, mut color) in &mut names {
        let Some(actor) = state.actor(CombatantKey::party(marker.0)) else {
            continue;
        };
        color.0 = if actor.is_alive() {
            battle_ink()
        } else {
            battle_dim()
        };
    }
}

fn sync_party_meters(
    state: Option<Res<BattleState>>,
    mut fills: Query<(&BattlePartyMeterFill, &mut Node, &mut BackgroundColor)>,
    mut labels: Query<(&BattlePartyMeterText, &mut Text)>,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    for (marker, mut node, mut color) in &mut fills {
        let Some(actor) = state.actor(CombatantKey::party(marker.0.index)) else {
            continue;
        };
        let (value, maximum) = match marker.0.kind {
            BattleMeterKind::Health => (actor.health, actor.max_health),
            BattleMeterKind::Mana => (actor.mana, actor.max_mana),
        };
        node.width = percent(meter_percent(value, maximum));
        color.0 = meter_color(marker.0.kind, value, maximum);
    }
    for (marker, mut text) in &mut labels {
        let Some(actor) = state.actor(CombatantKey::party(marker.0.index)) else {
            continue;
        };
        let (value, maximum) = match marker.0.kind {
            BattleMeterKind::Health => (actor.health, actor.max_health),
            BattleMeterKind::Mana => (actor.mana, actor.max_mana),
        };
        text.0 = meter_label(marker.0.kind, value, maximum);
    }
}

fn sync_battle_commands(
    state: Option<Res<BattleState>>,
    mut titles: Query<(&BattlePanelTitle, &mut Text), Without<BattleCommandLabel>>,
    mut rows: Query<(&BattleCommandRow, &mut BackgroundColor, &mut BorderColor)>,
    mut labels: Query<(&BattleCommandLabel, &mut Text, &mut TextColor), Without<BattlePanelTitle>>,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    for (marker, mut text) in &mut titles {
        if marker.0 == BattlePanelKind::Command {
            text.0 = if state.phase == BattlePhase::Ability {
                state.active().map_or_else(
                    || "Abilities".to_owned(),
                    |actor| format!("{}'s Abilities", actor.name),
                )
            } else if state.phase == BattlePhase::Item {
                "Battle Items".to_owned()
            } else if state.phase == BattlePhase::Rewards {
                "Victory Rewards".to_owned()
            } else {
                state
                    .active_key()
                    .and_then(|key| state.actor(key))
                    .map_or_else(
                        || "Command".to_owned(),
                        |actor| format!("{}'s Turn", actor.name),
                    )
            };
        }
    }
    let ability_choices = state.battle_ability_indices();
    let ability_page = state.ability_index / COMMANDS.len() * COMMANDS.len();
    let item_page = state.item_index / COMMANDS.len() * COMMANDS.len();
    let reward_lines = state
        .rewards
        .as_ref()
        .map_or_else(Vec::new, super::rewards::BattleRewards::summary_lines);
    for (marker, mut background, mut border) in &mut rows {
        let selected = (state.phase == BattlePhase::Command && marker.0 == state.command_index)
            || (state.phase == BattlePhase::Ability
                && ability_page + marker.0 == state.ability_index)
            || (state.phase == BattlePhase::Item && item_page + marker.0 == state.item_index);
        background.0 = if selected {
            battle_row_active()
        } else {
            battle_row()
        };
        border.set_all(if selected {
            battle_border_active()
        } else {
            battle_row_border()
        });
    }
    for (marker, mut text, mut color) in &mut labels {
        if state.phase == BattlePhase::Ability {
            let ability = ability_choices
                .get(ability_page + marker.0)
                .and_then(|index| state.active()?.abilities.get(*index));
            let Some(ability) = ability else {
                text.0.clear();
                color.0 = battle_dim();
                continue;
            };
            let affordable = state
                .active()
                .is_some_and(|actor| actor.mana >= ability.mp_cost);
            text.0 = format!("{}  {} MP", ability.name, ability.mp_cost);
            color.0 = if affordable {
                battle_ink()
            } else {
                battle_dim()
            };
            continue;
        }
        if state.phase == BattlePhase::Item {
            let Some(item) = state.item_choices.get(item_page + marker.0) else {
                text.0.clear();
                color.0 = battle_dim();
                continue;
            };
            text.0 = format!("{}  x{}", item.name, item.quantity);
            color.0 = battle_ink();
            continue;
        }
        if state.phase == BattlePhase::Rewards {
            text.0 = reward_lines.get(marker.0).cloned().unwrap_or_default();
            color.0 = if marker.0 == 0 {
                battle_gold()
            } else {
                battle_ink()
            };
            continue;
        }
        let command = COMMANDS[marker.0];
        let available = state.command_available(command);
        text.0 = if available {
            command.label().to_owned()
        } else {
            format!("{}  --", command.label())
        };
        color.0 = if available {
            battle_ink()
        } else {
            battle_dim()
        };
    }
}

fn sync_battle_message(
    state: Option<Res<BattleState>>,
    mut messages: Query<(&mut Text, &mut TextColor), With<BattleMessageText>>,
    mut targets: Query<&mut Text, (With<BattleTargetText>, Without<BattleMessageText>)>,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    if let Ok((mut text, mut color)) = messages.single_mut() {
        text.0.clone_from(&state.message);
        color.0 = match state.active_key().map(|key| key.side) {
            Some(BattleSide::Enemy) => battle_ember(),
            _ => battle_teal(),
        };
    }
    if let Ok(mut text) = targets.single_mut() {
        text.0 = state
            .target
            .as_ref()
            .and_then(|selector| state.actor(selector.selected()))
            .map(|actor| format!("Target  {}", actor.name))
            .unwrap_or_default();
    }
}

fn sync_enemy_cards(
    state: Option<Res<BattleState>>,
    mut cards: Query<(&BattleEnemyCard, &mut Node)>,
    mut frames: Query<(&BattleEnemyFrame, &mut BorderColor)>,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    let selected_target = state.target.as_ref().map(TargetSelector::selected);
    for (marker, mut node) in &mut cards {
        let Some(enemy) = state.actor(CombatantKey::enemy(marker.0)) else {
            continue;
        };
        node.display = if enemy.is_alive() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (marker, mut border) in &mut frames {
        let key = CombatantKey::enemy(marker.0);
        border.set_all(if selected_target == Some(key) {
            battle_target()
        } else {
            Color::NONE
        });
    }
}

fn sync_enemy_meters(
    state: Option<Res<BattleState>>,
    mut fills: Query<(&BattleEnemyHpFill, &mut Node, &mut BackgroundColor)>,
    mut labels: Query<(&BattleEnemyLabel, &mut Text)>,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    for (marker, mut node, mut color) in &mut fills {
        let Some(enemy) = state.actor(CombatantKey::enemy(marker.0)) else {
            continue;
        };
        let health_percent = meter_percent(enemy.health, enemy.max_health);
        node.width = percent(health_percent);
        color.0 = if health_percent <= 25.0 {
            battle_ember()
        } else {
            enemy_health()
        };
    }
    for (marker, mut text) in &mut labels {
        let Some(enemy) = state.actor(CombatantKey::enemy(marker.0)) else {
            continue;
        };
        text.0.clone_from(&enemy.name);
    }
}

fn cleanup_battle(mut commands: Commands, entities: Query<Entity, With<BattleUi>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<BattleState>();
    commands.remove_resource::<BattleAssetState>();
    commands.remove_resource::<super::fx::BattleFxRouter>();
    commands.remove_resource::<super::fx::BattleAttackAnimations>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_idle_frame_faces_down() {
        assert_eq!(BATTLE_IDLE_TILE, 18);
    }

    #[test]
    fn battle_uses_dedicated_enemy_sheet() {
        assert_eq!(
            battle_enemy_atlas_path("goblin").unwrap().as_str(),
            "assets/sprites/enemies/goblin_battle.tsx"
        );
    }

    #[test]
    fn original_battle_layout_spacing_is_preserved() {
        assert_eq!(enemy_layout_offset(1, 0), (0.0, 0.0));
        assert_eq!(enemy_layout_offset(3, 0), (-110.0, -30.0));
        assert_eq!(enemy_layout_offset(3, 1), (0.0, 20.0));
        assert_eq!(enemy_layout_offset(5, 4), (160.0, -30.0));
    }

    #[test]
    fn enemy_feet_are_anchored_to_the_authored_ground() {
        assert_eq!(
            enemy_card_position(full_enemy_ground(), 64.0, 80.0, 1, 0),
            (600.0, 210.0)
        );
        let cave_ground = GroundRect {
            x: 120,
            y: 310,
            width: 910,
            height: 158,
        };
        assert_eq!(
            enemy_card_position(cave_ground, 64.0, 80.0, 1, 0),
            (600.0, 365.0)
        );
        let courtyard_ground = GroundRect {
            x: 40,
            y: 351,
            width: 1200,
            height: 117,
        };
        assert_eq!(
            enemy_card_position(courtyard_ground, 64.0, 80.0, 1, 0),
            (600.0, 370.0)
        );
    }

    #[test]
    fn breathing_squashes_only_two_pixels_per_cycle() {
        assert_eq!(breath_squash(0.0, 0), 0.0);
        assert_eq!(breath_squash(BREATH_PERIOD_SECONDS / 2.0, 0), 2.0);
        assert_eq!(breath_squash(BREATH_PERIOD_SECONDS, 0), 0.0);
        assert!((0.0..=BREATH_MAX_SQUASH).contains(&breath_squash(0.3, 4)));
    }

    #[test]
    fn the_party_panel_reserves_every_slot_regardless_of_party_size() {
        // Fixed width: recruiting a companion fills a slot, it never reflows the panel.
        assert_eq!(party_panel_width(), 612.0);
        assert_eq!(
            party_panel_width(),
            PARTY_SLOT_COUNT as f32 * PARTY_CARD_WIDTH
                + (PARTY_SLOT_COUNT - 1) as f32 * PARTY_CARD_GAP
                + PANEL_PADDING * 2.0
        );
    }

    #[test]
    fn the_bottom_row_seats_every_panel_at_its_reserved_width() {
        // Panel row padding is 8 per side, with an 8px gap between each column.
        let available = crate::gameplay_canvas::LOGICAL_CANVAS_WIDTH as f32 - 8.0 * 2.0;
        let log = available - party_panel_width() - 8.0 - COMMAND_PANEL_WIDTH - 8.0;
        assert!(
            log >= 240.0,
            "a full party and a fixed command panel must still leave the log room, got {log}"
        );
    }

    #[test]
    fn the_command_panel_is_wide_enough_for_its_longest_row_label() {
        // "Fortress Stance  12 MP" is the widest label the panel ever shows, at
        // 177px in Philosopher-Regular 18px. Chrome must not eat into that.
        const WIDEST_MEASURED_LABEL: f32 = 177.0;
        assert_eq!(COMMAND_ROW_CHROME, 62.0);
        assert_eq!(COMMAND_PANEL_WIDTH, 262.0);
        let usable = COMMAND_PANEL_WIDTH - COMMAND_ROW_CHROME;
        assert!(
            usable >= WIDEST_MEASURED_LABEL,
            "the longest ability row would clip: {usable}px usable"
        );
    }

    #[test]
    fn a_filled_and_an_empty_slot_share_the_same_cell_footprint() {
        // Occupied and reserved cells must line up exactly, or the row looks ragged.
        let filled = party_card_node(true);
        let empty = party_card_node(false);
        assert_eq!(filled.width, empty.width);
        assert_eq!(filled.height, empty.height);
        assert_eq!(filled.border, empty.border);
        assert_eq!(filled.width, px(PARTY_CARD_WIDTH));
        assert_eq!(filled.height, px(PARTY_CARD_HEIGHT));
    }

    #[test]
    fn an_idle_enemy_holds_the_resting_pose() {
        assert_eq!(BATTLE_IDLE_TILE, 18);
        assert_eq!(enemy_sprite_tile(None), BATTLE_IDLE_TILE);
        // The resting pose is the spellcast row's first frame, so an attack that has only just
        // begun casting is indistinguishable from idle — by design, not by accident.
        assert_eq!(
            enemy_sprite_tile(Some((AttackKind::Spellcast, 0.0))),
            BATTLE_IDLE_TILE
        );
    }

    #[test]
    fn an_attack_walks_its_whole_row_and_stops_on_the_last_frame() {
        // Thrust: row 6, 8 frames — tiles 54..=61.
        assert_eq!(enemy_sprite_tile(Some((AttackKind::Thrust, 0.0))), 54);
        assert_eq!(enemy_sprite_tile(Some((AttackKind::Thrust, 0.5))), 58);
        assert_eq!(enemy_sprite_tile(Some((AttackKind::Thrust, 1.0))), 61);
        // Spellcast: row 2, 7 frames — tiles 18..=24. One frame shorter, so the two rows must
        // never share a step schedule.
        assert_eq!(enemy_sprite_tile(Some((AttackKind::Spellcast, 0.5))), 21);
        assert_eq!(enemy_sprite_tile(Some((AttackKind::Spellcast, 1.0))), 24);

        // Every frame of both rows exists on the authored sheets (108 tiles, 9 columns).
        for progress in 0..=10 {
            for kind in [AttackKind::Thrust, AttackKind::Spellcast] {
                let tile = enemy_sprite_tile(Some((kind, progress as f32 / 10.0)));
                assert!(tile < 108, "{kind:?} at {progress}/10 ran off the sheet");
            }
        }
    }

    #[test]
    fn meters_clamp_and_switch_to_low_health_color() {
        assert_eq!(meter_percent(50, 100), 50.0);
        assert_eq!(meter_percent(200, 100), 100.0);
        assert_eq!(meter_percent(1, 0), 0.0);
        assert_eq!(
            meter_color(BattleMeterKind::Health, 25, 100),
            battle_ember()
        );
    }

    fn actions_from(normalized: &[crate::input_record::NormalizedAction]) -> ActionState {
        let mut actions = ActionState::default();
        actions.replace_with_normalized(normalized);
        actions
    }

    #[test]
    fn right_confirms_and_left_cancels_outside_target_selection() {
        use crate::input_record::NormalizedAction as Key;

        // `battle_input.py:80,93` — Right is a second confirm on the command and sub-menus.
        assert!(confirm_pressed(&actions_from(&[Key::Confirm])));
        assert!(confirm_pressed(&actions_from(&[Key::MenuRight])));
        assert!(!confirm_pressed(&actions_from(&[Key::MenuLeft])));
        assert!(!confirm_pressed(&actions_from(&[])));

        // `battle_input.py:85` — Left backs out of a sub-menu the way ESC does.
        assert!(cancel_pressed(&actions_from(&[Key::Back])));
        assert!(cancel_pressed(&actions_from(&[Key::MenuLeft])));
        assert!(!cancel_pressed(&actions_from(&[Key::MenuRight])));
        assert!(!cancel_pressed(&actions_from(&[])));
    }

    #[test]
    fn target_selection_cycles_horizontally_without_confirming() {
        use crate::input_record::NormalizedAction as Key;

        // Enemies stand in a row, so Left/Right walk it just as Up/Down do
        // (`battle_input.py:106-109`).
        assert_eq!(target_navigation(&actions_from(&[Key::MenuLeft])), Some(-1));
        assert_eq!(target_navigation(&actions_from(&[Key::MenuRight])), Some(1));
        assert_eq!(target_navigation(&actions_from(&[Key::MenuUp])), Some(-1));
        assert_eq!(target_navigation(&actions_from(&[Key::MenuDown])), Some(1));
        assert_eq!(target_navigation(&actions_from(&[])), None);
        // One frame is one step, whichever axis it arrived on.
        assert_eq!(
            target_navigation(&actions_from(&[Key::MenuUp, Key::MenuRight])),
            Some(-1)
        );
    }

    #[test]
    fn a_target_walks_the_pool_when_left_and_right_are_pressed() {
        use crate::input_record::NormalizedAction as Key;

        let mut selector = TargetSelector {
            group: TargetGroup::Enemy,
            eligible: vec![
                CombatantKey::enemy(0),
                CombatantKey::enemy(1),
                CombatantKey::enemy(2),
            ],
            selected: 0,
        };
        selector.navigate(target_navigation(&actions_from(&[Key::MenuRight])).unwrap());
        assert_eq!(selector.selected(), CombatantKey::enemy(1));
        selector.navigate(target_navigation(&actions_from(&[Key::MenuLeft])).unwrap());
        assert_eq!(selector.selected(), CombatantKey::enemy(0));
        // The row wraps at both ends, so a leftmost enemy is one Left from the rightmost.
        selector.navigate(target_navigation(&actions_from(&[Key::MenuLeft])).unwrap());
        assert_eq!(selector.selected(), CombatantKey::enemy(2));
    }

    /// Drives the real spawn-and-sync pair, because a badge that exists only in the palette is the
    /// same bug as no badge at all: this is the wiring that was missing, not the colors.
    #[test]
    fn a_spawned_badge_stack_shows_exactly_the_statuses_a_combatant_carries() {
        use crate::battle::status::{ActiveStatus, StatusEffect};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, sync_status_badges);

        let font = Handle::<Font>::default();
        app.world_mut()
            .commands()
            .spawn(Node::default())
            .with_children(|root| {
                spawn_status_badges(root, CombatantKey::party(0), &font);
                spawn_status_badges(root, CombatantKey::enemy(0), &font);
            });
        app.update();

        // Nothing applied yet: every slot is spawned, and every slot is hidden.
        let slots = app
            .world_mut()
            .query::<(&BattleStatusBadge, &Node)>()
            .iter(app.world())
            .count();
        assert_eq!(slots, crate::battle::badge::MAX_BADGES * 2);
        assert!(
            app.world_mut()
                .query::<(&BattleStatusBadge, &Node)>()
                .iter(app.world())
                .all(|(_, node)| node.display == Display::None)
        );

        let mut party = crate::battle::tests::actor(BattleSide::Party, 0, 5, 20);
        party.status_effects = vec![
            ActiveStatus::damage_over_time(StatusEffect::Poison, Some(3), 4),
            ActiveStatus::timed(StatusEffect::Silence, 2),
        ];
        let mut enemy = crate::battle::tests::actor(BattleSide::Enemy, 0, 4, 30);
        enemy.status_effects = vec![ActiveStatus::modifier(
            StatusEffect::AttackModifier,
            2,
            0.85,
        )];
        app.world_mut()
            .insert_resource(crate::battle::tests::state_with(vec![party, enemy]));
        app.update();

        let labels = |key: CombatantKey, app: &mut App| {
            let mut query = app
                .world_mut()
                .query::<(&BattleStatusBadgeLabel, &Text, &Node)>();
            let mut rows = query
                .iter(app.world())
                .filter(|(marker, ..)| marker.key == key)
                .map(|(marker, text, _)| (marker.slot, text.0.clone()))
                .collect::<Vec<_>>();
            rows.sort_by_key(|(slot, _)| *slot);
            rows.into_iter().map(|(_, label)| label).collect::<Vec<_>>()
        };
        assert_eq!(
            labels(CombatantKey::party(0), &mut app),
            vec!["PSN".to_owned(), "SIL".to_owned(), String::new()],
            "both afflictions show, in the order they landed, and the spare slot stays blank"
        );
        assert_eq!(
            labels(CombatantKey::enemy(0), &mut app),
            vec!["ATK-".to_owned(), String::new(), String::new()],
            "an enemy weakened by Shield Bash wears the badge too, not just the party"
        );

        // Filled slots are visible and carry their palette fill; the spare stays hidden.
        let mut query = app
            .world_mut()
            .query::<(&BattleStatusBadge, &Node, &BackgroundColor)>();
        for (marker, node, background) in query.iter(app.world()) {
            let expected_filled = matches!(
                (marker.key.side, marker.slot),
                (BattleSide::Party, 0 | 1) | (BattleSide::Enemy, 0)
            );
            assert_eq!(
                node.display == Display::Flex,
                expected_filled,
                "{:?} slot {} visibility",
                marker.key,
                marker.slot
            );
            assert_eq!(
                background.0 != Color::NONE,
                expected_filled,
                "{:?} slot {} fill",
                marker.key,
                marker.slot
            );
        }
    }

    #[test]
    fn a_defeated_combatant_drops_its_badges() {
        use crate::battle::status::{ActiveStatus, StatusEffect};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, sync_status_badges);
        let font = Handle::<Font>::default();
        app.world_mut()
            .commands()
            .spawn(Node::default())
            .with_children(|root| spawn_status_badges(root, CombatantKey::party(0), &font));

        // A downed member is drawn dimmed, and a poison badge on a corpse reads as still ticking.
        let mut party = crate::battle::tests::actor(BattleSide::Party, 0, 5, 20);
        party.status_effects = vec![ActiveStatus::damage_over_time(
            StatusEffect::Poison,
            Some(3),
            4,
        )];
        party.health = 0;
        app.world_mut()
            .insert_resource(crate::battle::tests::state_with(vec![party]));
        app.update();

        assert!(
            app.world_mut()
                .query::<(&BattleStatusBadge, &Node)>()
                .iter(app.world())
                .all(|(_, node)| node.display == Display::None)
        );
    }

    #[test]
    fn battle_ui_sync_systems_have_disjoint_component_access() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<PlaySfx>()
            .add_systems(
                Update,
                (
                    sync_party_cards,
                    sync_party_meters,
                    sync_status_badges,
                    sync_battle_commands,
                    sync_battle_message,
                    sync_enemy_cards,
                    sync_enemy_meters,
                    super::super::reward_modal::sync_reward_modal,
                    super::super::fx::route_battle_fx,
                    super::super::fx::animate_battle_fx,
                    super::super::fx::animate_battle_shake,
                )
                    .chain(),
            );
        app.update();
    }
}
