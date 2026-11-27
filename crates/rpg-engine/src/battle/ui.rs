use bevy::ecs::system::SystemParam;
use bevy::{asset::RecursiveDependencyLoadState, prelude::*};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    battle_log::{BattleLogLine, LastBattleLog, LogKind, spawn_log_line},
    battle_tactics::{AutoCommand, TurnPause},
    debug_launch::{DebugBattleFixture, DebugLaunchConfig},
    encounter::{BattleEntry, BattleSide, restore_pre_battle_context},
    field_menu_domain::FieldMenuCatalog,
    game_state::GameState,
    input_bindings::InputBindings,
    key_hints::{HintKey, HintSegment, HintStyle, hint_keys_text, key_hint},
    options_domain::Device,
    options_store::PlayerOptions,
    runtime_party::RuntimeParty,
    scenario_balance::BalanceData,
    scenario_battle_background::{BattleBackgroundCatalog, GroundRect},
    scenario_encounter::EnemyRows,
    scenario_enemy::EnemySize,
    scenario_inventory::ScenarioInventory,
    scenario_new_game_assets::ActiveNewGameInputs,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    sfx_cue::{MenuSfx, PlaySfx, cue},
    spell_images::SpellIcon,
    tsx_atlas_asset::TsxAtlasAsset,
    world_audio::WorldBgmFadeInRequest,
    world_encounter::{BattleBgmFadeOut, WorldEncounterRestore},
};

use super::{
    ability::{AbilityError, AbilityTargetPlan, resolve_ability, target_plan},
    item::{ItemUseError, auto_items, battle_item, resolve_item, target_plan as item_target_plan},
    model::{
        BattleCommand, BattleItemChoice, BattlePhase, BattleState, CombatantKey, FleeOutcome,
        TargetGroup, TargetSelector,
    },
    party_ai::{AutoAction, plan_auto_action},
    rewards::apply_rewards,
    rules::{flee_chance, phase_after_flee_confirmation, roll_flee, wrap_index},
};

mod deck;
mod input;

use input::handle_battle_input;

const ENEMY_AREA_HEIGHT: f32 = 468.0;
const ENEMY_CANVAS_WIDTH: f32 = 1280.0;
const ENEMY_GROUND_NUDGE: f32 = 10.0;
/// Keep the health bar inside the clipped enemy area; status cards overlay the sprite.
const ENEMY_FOOTER_RESERVE: f32 = 32.0;
const BREATH_TOP_FRACTION: f32 = 0.60;
const BREATH_MAX_SQUASH: f32 = 2.0;
const BREATH_PERIOD_SECONDS: f32 = 1.4;
const BREATH_PHASE_OFFSET: f32 = 0.6;
/// Global stacking layer of the battle UI root.
///
/// `bevy_ui::stack` walks each root's children filtered by `Without<GlobalZIndex>`: a descendant
/// that carries one is lifted *out* of the walk and re-sorted against the roots themselves. So a
/// `GlobalZIndex` below this value does not lower a node within the battle screen — it drops the
/// node behind the whole screen, where the root's opaque [`battle_floor`] hides it however
/// correctly it laid out. Anything inside a frame that only needs to sit above its siblings uses a
/// plain [`ZIndex`]; only a node that must escape the frame it annotates goes global, and then it
/// goes *above* this.
pub(super) const BATTLE_ROOT_Z: i32 = 200;
/// Layer of a combatant's badge stack within its own frame, above the portrait and the meters.
const BADGE_Z: i32 = 15;
const BADGE_INSET: f32 = 5.0;
const STATUS_ICON_SIZE: f32 = 24.0;

/// Where a combatant's badge stack sits on its card.
#[derive(Clone, Copy, PartialEq, Debug)]
enum BadgeAnchor {
    /// Pinned into the top-right corner of the frame it annotates, stacked downwards.
    FrameCorner,
    /// Centered along the sprite tile's top edge, scaled to its displayed width.
    SpriteTopCenter { sprite_size: f32 },
}
const PARTY_PORTRAIT_SIZE: f32 = 137.6;
const PARTY_CARD_WIDTH: f32 = 147.6;
const PARTY_CARD_HEIGHT: f32 = 240.0;
const PARTY_CARD_GAP: f32 = 10.0;
/// Every slot the party can ever fill, drawn whether or not it is occupied.
///
/// Read from [`RuntimeParty::MAX_MEMBERS`] rather than restated, so a party that grew past what
/// this panel can seat is impossible by construction instead of merely unlikely.
const PARTY_SLOT_COUNT: usize = RuntimeParty::MAX_MEMBERS;
const ABILITY_ICON_SIZE: f32 = 36.0;
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
            .add_message::<BattleBgmFadeOut>()
            .init_resource::<LastBattleLog>()
            .add_systems(OnEnter(AppState::Battle), enter_battle)
            .add_systems(
                Update,
                (
                    drive_battle_assets,
                    position_enemy_cards,
                    animate_enemy_breathing,
                    deck::handle_pointer,
                    handle_battle_input,
                    request_battle_bgm_fade,
                    super::fx::route_battle_fx,
                    super::fx::animate_battle_fx,
                    super::fx::animate_battle_shake,
                    super::fx::animate_battle_dissolve,
                    sync_party_cards,
                    sync_party_meters,
                    sync_status_badges,
                    sync_battle_commands,
                    (
                        deck::sync_picker,
                        deck::sync_groups,
                        deck::sync_progress,
                        deck::sync_log_overlay,
                    )
                        .chain(),
                    sync_battle_message,
                    sync_enemy_cards,
                    sync_enemy_meters,
                    super::reward_modal::sync_reward_modal,
                    super::reward_modal::handle_reward_scroll,
                )
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            )
            .add_systems(
                PostUpdate,
                (
                    super::reward_modal::update_reward_scroll_chrome,
                    super::reward_modal::sync_reward_portraits,
                )
                    .after(bevy::ui::UiSystems::Layout)
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
    /// The whole pose, drawn black as the cast shadow.
    Shadow,
}

#[derive(Component)]
pub(super) struct BattleEnemyImage {
    pub(super) index: usize,
    part: EnemySpritePart,
    split_fraction: f32,
}

impl BattleEnemyImage {
    /// The tint at full presence: the body as painted, the shadow as a translucent black. The
    /// death fade scales its alpha.
    pub(super) fn full_color(&self) -> Color {
        match self.part {
            EnemySpritePart::Upper | EnemySpritePart::Lower => Color::WHITE,
            EnemySpritePart::Shadow => Color::srgba(0.0, 0.0, 0.0, ENEMY_SHADOW_OPACITY),
        }
    }
}

/// The node a cast shadow hangs from, nudged sideways with its enemy's hurt shake or dodge so the
/// shadow stays under the feet.
#[derive(Component)]
pub(super) struct BattleEnemyShadowShift(pub(super) usize);

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

/// The note the command panel shows in place of its cards once the battle is won.
#[derive(Component)]
struct BattleCommandIdle;

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
    icon_size: f32,
}

#[derive(Clone, Copy, Component)]
struct BattleStatusBadgeLabel {
    key: CombatantKey,
    slot: usize,
    icon_size: f32,
}

#[derive(Component)]
struct BattleMessageText;

/// The scrolling column the battle log's entries are appended to.
#[derive(Component)]
struct BattleLogList;

/// What the log has already recorded, so each new result is appended once.
#[derive(Default)]
struct BattleLogHistory {
    last: String,
    rewards_logged: bool,
}

#[derive(Component)]
struct BattleTargetText;

#[derive(Debug, Resource)]
pub(super) struct BattleAssetState {
    /// The v2 status sheet per enemy. One that is missing or fails to load draws no body; the
    /// battle still plays.
    status_atlases: Vec<Option<Handle<TsxAtlasAsset>>>,
    /// The optional v2 attack sheet per enemy, used only alongside that enemy's status sheet.
    attack_atlases: Vec<Option<Handle<TsxAtlasAsset>>>,
    /// Which enemies ship a v2 status sheet; any one switches the battle to the depth layout.
    v2_enemies: Vec<bool>,
    backgrounds: Handle<BattleBackgroundCatalog>,
    pub(super) font: Handle<Font>,
}

#[cfg(test)]
impl BattleAssetState {
    /// An asset set with nothing loaded, for systems that only read the font handle.
    pub(super) fn test_stub() -> Self {
        Self {
            status_atlases: Vec::new(),
            attack_atlases: Vec::new(),
            v2_enemies: Vec::new(),
            backgrounds: Handle::default(),
            font: Handle::default(),
        }
    }
}

/// Fill all badge slots for the session-only layout review without changing normal battles.
fn apply_layout_review_statuses(state: &mut BattleState, fixture: Option<DebugBattleFixture>) {
    use super::status::{ActiveStatus, StatusEffect};

    if !matches!(
        fixture,
        Some(DebugBattleFixture::LayoutThree | DebugBattleFixture::LayoutFive)
    ) {
        return;
    }
    for actor in &mut state.combatants {
        let speed = if actor.key.side == BattleSide::Party {
            1.5
        } else {
            0.5
        };
        actor.status_effects = vec![
            ActiveStatus::speed(99, speed),
            ActiveStatus::regeneration(99, 0.05),
            ActiveStatus::modifier(StatusEffect::DefenseModifier, 99, 1.5),
        ];
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "battle entry loads scenario UI resources and optional debug review settings"
)]
fn enter_battle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    catalog: Res<FieldMenuCatalog>,
    game: Res<GameState>,
    entry: Option<Res<BattleEntry>>,
    debug: Option<Res<DebugLaunchConfig>>,
    balances: Res<Assets<BalanceData>>,
    active: Res<ActiveNewGameInputs>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(entry) = entry else {
        return;
    };
    let mut state = BattleState::from_entry(&entry);
    if let Some(balance) = active.balance(&balances) {
        state.party_ai = balance.party_ai;
        state.status_damage = balance.status_damage;
    }
    apply_layout_review_statuses(
        &mut state,
        debug.as_ref().and_then(|debug| debug.battle_fixture),
    );
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
    let v2_enemies = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .map(|participant| {
            super::enemy_pose::status_sheet_path(&participant.sprite_id)
                .is_ok_and(|path| inventory.enemy_battler_sheets.contains(&path))
        })
        .collect::<Vec<_>>();
    let status_atlases = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .map(|participant| {
            super::enemy_pose::status_sheet_path(&participant.sprite_id)
                .ok()
                .filter(|path| inventory.enemy_battler_sheets.contains(path))
                .map(|path| asset_server.load(root.resolve(&path)))
        })
        .collect::<Vec<_>>();
    let attack_atlases = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .map(|participant| {
            super::enemy_pose::attack_sheet_path(&participant.sprite_id)
                .ok()
                .filter(|path| inventory.enemy_battler_sheets.contains(path))
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
            GlobalZIndex(BATTLE_ROOT_Z),
            Pickable::IGNORE,
            BattleUi,
        ))
        .with_children(|root_node| {
            spawn_enemy_area(root_node, &entry, &v2_enemies, background, &font);
            deck::spawn_log_overlay(root_node, &font);
            deck::spawn_panels(
                root_node,
                &entry,
                &game,
                &catalog,
                &asset_server,
                &root,
                &font,
                &mut images,
            );
        });
    commands.insert_resource(BattleAssetState {
        status_atlases,
        attack_atlases,
        v2_enemies,
        backgrounds,
        font,
    });
    commands.insert_resource(super::fx::BattleFxRouter::default());
    commands.insert_resource(super::fx::BattleAttackAnimations::default());
    commands.insert_resource(deck::BattleDeck::default());
    commands.insert_resource(state);
}

fn spawn_enemy_area(
    parent: &mut ChildSpawnerCommands<'_>,
    entry: &BattleEntry,
    v2_enemies: &[bool],
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
                // Every shadow first, so they lie on the ground under all the bodies: a front
                // enemy's shadow must not paint over the enemy behind it.
                for (index, participant) in enemies.iter().enumerate() {
                    let layout = enemy_card_layout(
                        full_enemy_ground(),
                        participant,
                        v2_enemies,
                        entry.enemy_rows,
                        enemies.len(),
                        index,
                    );
                    spawn_enemy_cast_shadow(battlefield, index, layout);
                }
                for (index, participant) in enemies.iter().enumerate() {
                    let layout = enemy_card_layout(
                        full_enemy_ground(),
                        participant,
                        v2_enemies,
                        entry.enemy_rows,
                        enemies.len(),
                        index,
                    );
                    let sprite_size = layout.sprite_size;
                    battlefield
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(layout.left),
                                top: px(layout.top),
                                width: px(layout.card_width),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ZIndex(layout.z),
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
                                        spawn_status_badges(
                                            sprite,
                                            CombatantKey::enemy(index),
                                            BadgeAnchor::SpriteTopCenter { sprite_size },
                                            font,
                                        );
                                    });
                            });
                            card.spawn((
                                Node {
                                    position_type: PositionType::Relative,
                                    width: layout.bar_width.map_or(percent(100), px),
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

fn spawn_party_cards(
    parent: &mut ChildSpawnerCommands<'_>,
    party: &[&crate::encounter::BattleParticipant],
    game: &GameState,
    catalog: &FieldMenuCatalog,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    font: &Handle<Font>,
) {
    parent
        .spawn(Node {
            width: percent(100),
            height: percent(100),
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
                        spawn_party_card(
                            row,
                            index,
                            participant,
                            game,
                            catalog,
                            asset_server,
                            root,
                            font,
                        );
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
            JustifyContent::SpaceBetween
        } else {
            JustifyContent::Center
        },
        ..default()
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "party cards combine portrait and battle data with persistent level and EXP"
)]
fn spawn_party_card(
    row: &mut ChildSpawnerCommands<'_>,
    index: usize,
    participant: &crate::encounter::BattleParticipant,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    font: &Handle<Font>,
) {
    let portrait = ScenarioRelativePath::try_from(
        format!("media/images/{}_profile.png", participant.id).as_str(),
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
        card.spawn(Node {
            width: percent(100),
            height: px(22),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|name| {
            spawn_battle_text(
                name,
                &participant.name,
                font,
                17.0,
                battle_ink(),
                Justify::Left,
            )
            .insert(BattlePartyName(index));
            let level = game
                .party()
                .member(&participant.id)
                .map_or(1, |member| member.level());
            spawn_battle_text(
                name,
                format!("Lv {level}"),
                font,
                13.0,
                Color::srgb_u8(191, 178, 155),
                Justify::Right,
            )
            .insert(deck::PartyLevel(index));
        });
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
        deck::spawn_experience(card, index, &participant.id, game, catalog, font);
        spawn_status_badges(
            card,
            CombatantKey::party(index),
            BadgeAnchor::FrameCorner,
            font,
        );
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
                height: px(22),
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
                13.0,
                battle_ink(),
                Justify::Center,
            )
            .insert(BattlePartyMeterText(marker));
        });
}

/// Lays out the combatant's status pills where [`BadgeAnchor`] puts them.
///
/// Every slot is spawned up front and hidden; [`sync_status_badges`] only ever toggles and
/// updates their artwork or fallback text. The absolute stack never changes the card's layout.
///
/// The stack sits inside its own parent, so it is layered with a plain [`ZIndex`] and not a
/// `GlobalZIndex` — see [`BATTLE_ROOT_Z`] for why the global form would have hidden it behind the
/// battle screen entirely.
fn spawn_status_badges(
    parent: &mut ChildSpawnerCommands<'_>,
    key: CombatantKey,
    anchor: BadgeAnchor,
    font: &Handle<Font>,
) {
    let (icon_size, node) = match anchor {
        BadgeAnchor::FrameCorner => (
            STATUS_ICON_SIZE,
            Node {
                position_type: PositionType::Absolute,
                top: px(BADGE_INSET),
                right: px(BADGE_INSET),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: px(2),
                ..default()
            },
        ),
        BadgeAnchor::SpriteTopCenter { sprite_size } => (
            (sprite_size * 0.20).floor().max(16.0),
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: percent(100),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                column_gap: px(1),
                ..default()
            },
        ),
    };
    parent
        .spawn((node, ZIndex(BADGE_Z), Pickable::IGNORE))
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
                        ImageNode {
                            color: Color::NONE,
                            ..default()
                        },
                        BattleStatusBadge {
                            key,
                            slot,
                            icon_size,
                        },
                    ))
                    .with_children(|pill| {
                        spawn_battle_text(
                            pill,
                            "",
                            font,
                            (icon_size * 0.38).max(11.0),
                            battle_ink(),
                            Justify::Center,
                        )
                        .insert(BattleStatusBadgeLabel {
                            key,
                            slot,
                            icon_size,
                        });
                    });
            }
        });
}

/// Shows active statuses as icons, retaining text for effects without artwork.
///
/// Runs off the same `is_changed` gate as the other sync systems: statuses only move when the
/// resolver writes them, so there is nothing to recompute on a quiet frame.
fn sync_status_badges(
    state: Option<Res<BattleState>>,
    asset_server: Option<Res<AssetServer>>,
    root: Option<Res<ScenarioRoot>>,
    inventory: Option<Res<ScenarioInventory>>,
    mut pills: Query<(
        &BattleStatusBadge,
        &mut Node,
        &mut BackgroundColor,
        &mut ImageNode,
    )>,
    mut labels: Query<
        (
            &BattleStatusBadgeLabel,
            &mut Text,
            &mut TextColor,
            &mut Node,
            &mut TextFont,
        ),
        Without<BattleStatusBadge>,
    >,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    let badges_for = |key: CombatantKey| {
        state
            .actor(key)
            .map(|actor| {
                if actor.is_alive() {
                    super::badge::badges(&actor.status_effects)
                } else if key.side == BattleSide::Party {
                    vec![super::badge::knocked_out_badge()]
                } else {
                    Vec::new()
                }
            })
            .unwrap_or_default()
    };
    let image_for = |badge: super::badge::StatusBadge| -> Option<Handle<Image>> {
        let path = inventory.as_ref()?.status_images.get(badge.icon_id()?)?;
        Some(asset_server.as_ref()?.load(root.as_ref()?.resolve(path)))
    };
    for (marker, mut node, mut background, mut image) in &mut pills {
        let badge = badges_for(marker.key).get(marker.slot).copied();
        let handle = badge.and_then(image_for);
        let has_image = handle.is_some();
        node.display = if badge.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        node.width = if has_image {
            px(marker.icon_size)
        } else {
            Val::Auto
        };
        node.height = if has_image {
            px(marker.icon_size)
        } else {
            Val::Auto
        };
        node.flex_shrink = 0.0;
        node.padding = if has_image {
            UiRect::ZERO
        } else {
            UiRect::axes(px(4), px(1))
        };
        image.image = handle.unwrap_or_default();
        image.color = if has_image { Color::WHITE } else { Color::NONE };
        background.0 = if has_image {
            Color::srgba(0.0, 0.0, 0.0, 0.5)
        } else {
            badge.map_or(Color::NONE, |badge| badge.fill)
        };
    }
    for (marker, mut text, mut color, mut node, mut font) in &mut labels {
        let badge = badges_for(marker.key).get(marker.slot).copied();
        let has_image = badge.and_then(image_for).is_some();
        font.font_size = if has_image {
            (marker.icon_size * 0.38).max(11.0)
        } else {
            11.0
        }
        .into();
        text.0 = match badge {
            Some(badge) if has_image => badge
                .remaining_turns
                .map(|turns| turns.to_string())
                .unwrap_or_default(),
            Some(badge) => badge
                .remaining_turns
                .map(|turns| format!("{} {turns}", badge.label))
                .unwrap_or_else(|| badge.label.to_owned()),
            None => String::new(),
        };
        node.position_type = if has_image {
            PositionType::Absolute
        } else {
            PositionType::Relative
        };
        node.bottom = if has_image { px(0) } else { Val::Auto };
        node.right = if has_image { px(2) } else { Val::Auto };
        color.0 = if has_image {
            Color::WHITE
        } else {
            badge.map_or(Color::NONE, |badge| badge.ink)
        };
    }
}

fn spawn_command_rows(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    parent
        .spawn(Node {
            width: percent(100),
            height: px(34),
            column_gap: px(8),
            position_type: PositionType::Relative,
            ..default()
        })
        .with_children(|list| {
            list.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Visibility::Hidden,
                BattleCommandIdle,
            ))
            .with_children(|note| {
                spawn_battle_text(
                    note,
                    "Battle won",
                    font,
                    17.0,
                    battle_gold(),
                    Justify::Center,
                );
            });
            for (index, command) in COMMANDS.iter().enumerate() {
                list.spawn((
                    Node {
                        flex_basis: px(0),
                        flex_grow: 1.0,
                        height: percent(100),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
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
                        17.0,
                        battle_ink(),
                        Justify::Center,
                    )
                    .insert(BattleCommandLabel(index));
                });
            }
        });
}

/// The log keeps every result of the battle in a scrolling list, newest at the bottom, with the
/// current prompt beneath it. Prompts ("choose an action") are not history and are never kept.
fn spawn_message_log(parent: &mut ChildSpawnerCommands<'_>) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            ..default()
        })
        .with_children(|log| {
            log.spawn((
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                ScrollPosition::default(),
                BattleLogList,
            ));
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
    count * PARTY_CARD_WIDTH + (count - 1.0) * PARTY_CARD_GAP
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
    let ground = usable_ground(ground);
    let (offset_x, offset_y) = enemy_layout_offset(enemy_count, index);
    let center_y = ground.y as f32 + ground.height as f32 / 2.0 + ENEMY_GROUND_NUDGE + offset_y;
    place_enemy_card(
        ground,
        sprite_size,
        card_width,
        ENEMY_CANVAS_WIDTH / 2.0 + offset_x,
        center_y + sprite_size / 2.0,
    )
}

fn usable_ground(ground: GroundRect) -> GroundRect {
    if ground.width > 0 && ground.height > 0 {
        ground
    } else {
        full_enemy_ground()
    }
}

/// Clamps a card's sprite center and tile bottom onto the background's ground, keeping the
/// health bar under the feet inside the enemy area.
fn place_enemy_card(
    ground: GroundRect,
    sprite_size: f32,
    card_width: f32,
    center_x: f32,
    feet: f32,
) -> (f32, f32) {
    let ground_left = ground.x as f32;
    let ground_top = ground.y as f32;
    let ground_right = ground_left + ground.width as f32;
    let ground_bottom = ground_top + ground.height as f32;

    let minimum_x = ground_left + sprite_size / 2.0;
    let maximum_x = (ground_right - sprite_size / 2.0).max(minimum_x);
    let center_x = center_x.clamp(minimum_x, maximum_x);

    let minimum_feet = ground_top;
    let maximum_feet = (ground_bottom - ENEMY_FOOTER_RESERVE).max(minimum_feet);
    let feet = feet.clamp(minimum_feet, maximum_feet);

    (center_x - card_width / 2.0, feet - sprite_size - 2.0)
}

/// Where one enemy card sits, how big its sprite is drawn, and how it stacks.
#[derive(Clone, Copy, Debug, PartialEq)]
struct EnemyCardLayout {
    sprite_size: f32,
    card_width: f32,
    left: f32,
    top: f32,
    z: i32,
    /// A fixed health-bar width; `None` spans the card.
    bar_width: Option<f32>,
}

/// An enemy's cast shadow: its own current pose, laid on the ground from the feet toward the
/// north-east, the way the light falls on the field map. A point `h` above the feet lands
/// `LEAN * h` right and `LENGTH * h` up the screen.
const ENEMY_SHADOW_LEAN: f32 = 0.7;
const ENEMY_SHADOW_LENGTH: f32 = 0.45;
/// How far above the tile's bottom edge the painted feet land.
const ENEMY_SHADOW_FEET_INSET: f32 = 10.0;
/// How dark the cast shadow is. Hard-edged, so lighter than the soft field blob.
const ENEMY_SHADOW_OPACITY: f32 = 0.55;

/// The cast shadow's nested rotate-scale-rotate and the translation that pins its pivot to the
/// feet rather than the tile centre, where UI transforms turn.
fn enemy_shadow_slant(sprite_size: f32) -> (crate::blob_shadow::NestedLinear, Vec2) {
    let slant = crate::blob_shadow::cast_shadow_y_down(ENEMY_SHADOW_LEAN, ENEMY_SHADOW_LENGTH);
    let feet_below_centre = Vec2::new(0.0, sprite_size / 2.0 - ENEMY_SHADOW_FEET_INSET);
    (
        crate::blob_shadow::NestedLinear::from_mat2(slant),
        feet_below_centre - slant * feet_below_centre,
    )
}

/// Lays one enemy's cast shadow on the ground. The holder sits exactly where the enemy's card
/// does (and carries its [`BattleEnemyCard`] marker, so it moves and hides with the card); inside,
/// two nested nodes over the sprite tile shear the pose image about the feet. Neither UI transform
/// shears on its own, so the slant is split into rotate-scale (outer) and rotate (inner).
fn spawn_enemy_cast_shadow(
    battlefield: &mut ChildSpawnerCommands<'_>,
    index: usize,
    layout: EnemyCardLayout,
) {
    let sprite_size = layout.sprite_size;
    let (parts, shift) = enemy_shadow_slant(sprite_size);
    battlefield
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(layout.left),
                top: px(layout.top),
                width: px(layout.card_width),
                height: px(sprite_size + 4.0),
                ..default()
            },
            Pickable::IGNORE,
            BattleEnemyCard(index),
        ))
        .with_children(|holder| {
            holder
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    Pickable::IGNORE,
                    BattleEnemyShadowShift(index),
                ))
                .with_children(|nudge| {
                    nudge
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px((layout.card_width - sprite_size) / 2.0),
                                top: px(2.0),
                                width: px(sprite_size),
                                height: px(sprite_size),
                                ..default()
                            },
                            UiTransform {
                                translation: Val2::px(shift.x, shift.y),
                                scale: parts.scale,
                                rotation: Rot2::radians(parts.outer_rotation),
                            },
                            Pickable::IGNORE,
                        ))
                        .with_child((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(0),
                                top: px(0),
                                width: percent(100),
                                height: percent(100),
                                ..default()
                            },
                            UiTransform::from_rotation(Rot2::radians(parts.inner_rotation)),
                            ImageNode::solid_color(Color::NONE),
                            Pickable::IGNORE,
                            BattleEnemyImage {
                                index,
                                part: EnemySpritePart::Shadow,
                                split_fraction: 1.0,
                            },
                        ));
                });
        });
}

/// The health bar under a v2 battler. Its card is as wide as the 256-300px tile, and a bar that
/// wide would run into its neighbours'.
const ENEMY_V2_BAR_WIDTH: f32 = 160.0;

/// Lays out one enemy card. A battle with any v2 enemy uses the depth rows, where v2 enemies take
/// their row's size and v1 enemies keep their own; otherwise the legacy presets apply unchanged.
fn enemy_card_layout(
    ground: GroundRect,
    participant: &crate::encounter::BattleParticipant,
    v2_enemies: &[bool],
    rows: EnemyRows,
    enemy_count: usize,
    index: usize,
) -> EnemyCardLayout {
    let ground = usable_ground(ground);
    if !v2_enemies.iter().any(|&v2| v2) {
        let sprite_size = enemy_sprite_size(participant);
        let card_width = sprite_size.max(80.0);
        let (left, top) = enemy_card_position(ground, sprite_size, card_width, enemy_count, index);
        return EnemyCardLayout {
            sprite_size,
            card_width,
            left,
            top,
            z: 0,
            bar_width: None,
        };
    }
    let (center_x, row) = super::enemy_depth::depth_slot(enemy_count, index, rows);
    let v2 = v2_enemies.get(index).copied().unwrap_or(false);
    let sprite_size = if v2 {
        row.sprite_size()
    } else {
        enemy_sprite_size(participant)
    };
    let card_width = sprite_size.max(80.0);
    let (left, top) =
        place_enemy_card(ground, sprite_size, card_width, center_x, row.tile_bottom());
    EnemyCardLayout {
        sprite_size,
        card_width,
        left,
        top,
        z: row.z(),
        bar_width: v2.then_some(ENEMY_V2_BAR_WIDTH.min(card_width)),
    }
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

/// The targeting reticle colour.
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

enum Sheet<'a> {
    Absent,
    Pending,
    Ready(&'a TsxAtlasAsset),
}

/// Where one enemy sheet stands. A sheet is ready only once its PNG has loaded too: a TSX whose
/// image is missing would otherwise count as loaded and draw an invisible body.
fn sheet_state<'a>(
    asset_server: &AssetServer,
    atlases: &'a Assets<TsxAtlasAsset>,
    handle: Option<&Option<Handle<TsxAtlasAsset>>>,
) -> Sheet<'a> {
    let Some(Some(handle)) = handle else {
        return Sheet::Absent;
    };
    match asset_server.recursive_dependency_load_state(handle.id()) {
        RecursiveDependencyLoadState::Failed(_) => Sheet::Absent,
        RecursiveDependencyLoadState::Loaded => {
            atlases.get(handle).map_or(Sheet::Pending, Sheet::Ready)
        }
        RecursiveDependencyLoadState::NotLoaded | RecursiveDependencyLoadState::Loading => {
            Sheet::Pending
        }
    }
}

fn drive_battle_assets(
    asset_server: Res<AssetServer>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    assets: Option<Res<BattleAssetState>>,
    state: Option<Res<BattleState>>,
    attacks: Option<Res<super::fx::BattleAttackAnimations>>,
    mut images: Query<(&BattleEnemyImage, &mut ImageNode)>,
) {
    let Some(assets) = assets else { return };
    for (marker, mut image) in &mut images {
        let animation = attacks
            .as_deref()
            .and_then(|attacks| attacks.progress(marker.index));
        let sheet = |handles: &[Option<Handle<TsxAtlasAsset>>]| {
            sheet_state(&asset_server, &atlases, handles.get(marker.index))
        };
        let actor = state
            .as_deref()
            .and_then(|state| state.actor(CombatantKey::enemy(marker.index)));
        // The v2 status sheet owns the body; an enemy without a loadable one draws nothing. A
        // swing shows the attack sheet when there is one and otherwise holds the status pose.
        let (atlas, tile) = match (sheet(&assets.status_atlases), actor) {
            (Sheet::Pending | Sheet::Absent, _) | (Sheet::Ready(_), None) => continue,
            (Sheet::Ready(status), Some(actor)) => {
                let alive = actor.is_alive();
                let swing = animation.filter(|_| alive).and_then(|(kind, _)| {
                    match sheet(&assets.attack_atlases) {
                        Sheet::Ready(attack) => {
                            Some((attack, super::enemy_pose::attack_tile(kind)))
                        }
                        Sheet::Absent | Sheet::Pending => None,
                    }
                });
                swing.unwrap_or_else(|| {
                    (
                        status,
                        super::enemy_pose::status_tile(alive, &actor.status_effects),
                    )
                })
            }
        };
        // A sheet with fewer tiles than the table expects shows its first pose rather than
        // freezing on whatever the body last drew.
        let sprite = atlas
            .sprite_for_tile(tile)
            .or_else(|_| atlas.sprite_for_tile(0));
        if let Ok(sprite) = sprite {
            image.image = sprite.image;
            image.texture_atlas = sprite.texture_atlas;
            let source_width = atlas.metadata().tile_width() as f32;
            let source_height = atlas.metadata().tile_height() as f32;
            let split = source_height * marker.split_fraction;
            image.rect = Some(match marker.part {
                EnemySpritePart::Upper => Rect::new(0.0, 0.0, source_width, split),
                EnemySpritePart::Lower => Rect::new(0.0, split, source_width, source_height),
                EnemySpritePart::Shadow => Rect::new(0.0, 0.0, source_width, source_height),
            });
            image.image_mode = NodeImageMode::Stretch;
            image.color = marker.full_color();
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
        let layout = enemy_card_layout(
            ground,
            participant,
            &assets.v2_enemies,
            entry.enemy_rows,
            enemies.len(),
            marker.0,
        );
        node.left = px(layout.left);
        node.top = px(layout.top);
    }
}

/// Idle breathing, held flat while an enemy is mid-swing so the squash does not fight the attack
/// frames — the same suspension the attack animation expects. A frozen enemy is held flat too:
/// ice does not breathe.
fn animate_enemy_breathing(
    time: Res<Time>,
    state: Option<Res<BattleState>>,
    attacks: Option<Res<super::fx::BattleAttackAnimations>>,
    mut upper_bodies: Query<(&BattleEnemyUpperBody, &mut Node)>,
) {
    for (marker, mut node) in &mut upper_bodies {
        let attacking = attacks
            .as_deref()
            .is_some_and(|attacks| attacks.progress(marker.index).is_some());
        let frozen = state.as_deref().is_some_and(|state| {
            state
                .actor(CombatantKey::enemy(marker.index))
                .is_some_and(|enemy| enemy.has_status(super::status::StatusEffect::Freeze))
        });
        let squash = if attacking || frozen {
            0.0
        } else {
            breath_squash(time.elapsed_secs(), marker.index)
        };
        node.top = px(squash);
        node.height = px((marker.base_height - squash).max(1.0));
    }
}

fn request_battle_bgm_fade(
    state: Option<Res<BattleState>>,
    mut fade_requests: MessageWriter<BattleBgmFadeOut>,
    mut requested: Local<bool>,
) {
    let defeated = state.is_some_and(|state| {
        let enemies = state
            .combatants
            .iter()
            .filter(|actor| actor.key.side == BattleSide::Enemy);
        let mut enemies = enemies.peekable();
        enemies.peek().is_some() && enemies.all(|actor| actor.health == 0)
    });
    if defeated && !*requested {
        fade_requests.write(BattleBgmFadeOut);
        *requested = true;
    } else if !defeated {
        *requested = false;
    }
}

/// Confirm, or Right — both are accepted on the command and sub-menus.
/// Target selection is deliberately excluded: there Right steps the
/// pool instead.
fn confirm_pressed(actions: &ActionState) -> bool {
    actions.just_pressed(AppAction::Confirm) || actions.just_pressed(AppAction::Right)
}

/// Back, or Left — Left backs out of a sub-menu.
fn cancel_pressed(actions: &ActionState) -> bool {
    actions.just_pressed(AppAction::Back) || actions.just_pressed(AppAction::Left)
}

/// One step through the target pool: Left/Up step back, Right/Down step forward.
fn target_navigation(actions: &ActionState) -> Option<isize> {
    actions
        .menu_navigation()
        .or_else(|| actions.menu_navigation_horizontal())
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
        background.0 = Color::srgb_u8(30, 30, 38);
        // A heal or a revive picks its target from these cards, so the reticle has to land on the
        // member the way it lands on an enemy sprite.
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
    mut rows: Query<(
        &BattleCommandRow,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Visibility,
    )>,
    mut idle: Query<&mut Visibility, (With<BattleCommandIdle>, Without<BattleCommandRow>)>,
    mut labels: Query<(&BattleCommandLabel, &mut TextColor)>,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    let finished = matches!(state.phase, BattlePhase::Victory | BattlePhase::Rewards);
    for mut visibility in &mut idle {
        *visibility = if finished {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (marker, mut background, mut border, mut visibility) in &mut rows {
        *visibility = if finished {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        let selected = marker.0 == state.command_index;
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
    for (marker, mut color) in &mut labels {
        color.0 = if state.command_available(COMMANDS[marker.0]) {
            battle_ink()
        } else {
            battle_dim()
        };
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the log feeds the on-screen list, the prompt, the target line, and the kept record"
)]
fn sync_battle_message(
    mut commands: Commands,
    state: Option<Res<BattleState>>,
    assets: Option<Res<BattleAssetState>>,
    mut history: Local<BattleLogHistory>,
    mut record: Option<ResMut<LastBattleLog>>,
    mut lists: Query<(Entity, &mut ScrollPosition), With<BattleLogList>>,
    mut messages: Query<&mut Text, With<BattleMessageText>>,
    mut targets: Query<&mut Text, (With<BattleTargetText>, Without<BattleMessageText>)>,
) {
    let Some(state) = state else { return };
    if state.is_added() {
        *history = BattleLogHistory::default();
        if let Some(record) = record.as_deref_mut() {
            record.clear();
        }
    }
    if !state.is_changed() {
        return;
    }
    let prompting = is_prompt_phase(state.phase);
    let enemy_names = state
        .combatants
        .iter()
        .filter(|actor| actor.key.side == BattleSide::Enemy)
        .map(|actor| actor.name.clone())
        .collect::<Vec<_>>();
    let mut lines = Vec::new();
    if !prompting && !state.message.is_empty() && state.message != history.last {
        let enemy = state.active_key().map(|key| key.side) == Some(BattleSide::Enemy);
        lines.push(BattleLogLine {
            text: state.message.clone(),
            enemy,
            kind: state.message_kind(),
            enemy_names: enemy_names.clone(),
        });
    }
    history.last.clone_from(&state.message);
    // Rewards land the frame they are applied, before the victory skip can leave the battle.
    if let Some(rewards) = state.rewards.as_ref().filter(|_| !history.rewards_logged) {
        history.rewards_logged = true;
        lines.extend(rewards.log_lines().into_iter().map(|text| BattleLogLine {
            text,
            enemy: false,
            kind: LogKind::Reward,
            enemy_names: Vec::new(),
        }));
    }
    for line in &lines {
        if let Some(record) = record.as_deref_mut() {
            record.push(line.clone());
        }
        // Let the reward modals reveal the spoils; retain the details for the field Battle Log.
        if line.kind == LogKind::Reward {
            continue;
        }
        if let (Ok((list, mut scroll)), Some(assets)) = (lists.single_mut(), assets.as_deref()) {
            commands.entity(list).with_children(|list| {
                spawn_log_line(
                    list,
                    line,
                    &assets.font,
                    16.0,
                    TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                );
            });
            // Layout clamps this to the real bottom, so the newest entry is always in view.
            scroll.0.y = f32::MAX;
        }
    }
    if let Ok(mut text) = messages.single_mut() {
        text.0 = if prompting {
            state.message.clone()
        } else {
            String::new()
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

/// Phases where the message is an instruction to the player rather than something that happened.
const fn is_prompt_phase(phase: BattlePhase) -> bool {
    matches!(
        phase,
        BattlePhase::Command | BattlePhase::Ability | BattlePhase::Item | BattlePhase::Target
    )
}

fn sync_enemy_cards(
    state: Option<Res<BattleState>>,
    mut cards: Query<(&BattleEnemyCard, &mut Node)>,
    mut frames: Query<(&BattleEnemyFrame, &mut BorderColor)>,
    dissolves: Query<(&BattleEnemyFrame, &super::fx::BattleDeathDissolve)>,
) {
    let Some(state) = state else { return };
    // A dissolving enemy is still being drawn, so this cannot idle on an unchanged state: the
    // card has to stay flexed for the whole fade and then be hidden on the frame it ends, which
    // is a change in this system's output with no change in `BattleState` behind it.
    let dissolving: Vec<usize> = dissolves
        .iter()
        .filter(|(_, dissolve)| dissolve.is_running())
        .map(|(frame, _)| frame.0)
        .collect();
    if !state.is_changed() && dissolving.is_empty() {
        return;
    }
    let selected_target = state.target.as_ref().map(TargetSelector::selected);
    for (marker, mut node) in &mut cards {
        let Some(enemy) = state.actor(CombatantKey::enemy(marker.0)) else {
            continue;
        };
        node.display = if enemy.is_alive() || dissolving.contains(&marker.0) {
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
    commands.remove_resource::<deck::BattleDeck>();
    commands.remove_resource::<BattleAssetState>();
    commands.remove_resource::<super::reward_modal::BattleRewardPrelude>();
    commands.remove_resource::<super::reward_modal::BattleRewardScreen>();
    commands.remove_resource::<super::fx::BattleFxRouter>();
    commands.remove_resource::<super::fx::BattleAttackAnimations>();
}

#[cfg(test)]
#[path = "ui/outcome_regressions.rs"]
mod outcome_regressions;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_reviews_fill_both_sides_badge_slots_without_changing_normal_battles() {
        let mut state = crate::battle::tests::state_with(vec![
            crate::battle::tests::actor(BattleSide::Party, 0, 10, 20),
            crate::battle::tests::actor(BattleSide::Enemy, 0, 10, 20),
        ]);
        for fixture in [None, Some(DebugBattleFixture::Status)] {
            apply_layout_review_statuses(&mut state, fixture);
            assert!(
                state
                    .combatants
                    .iter()
                    .all(|actor| actor.status_effects.is_empty())
            );
        }
        for fixture in [
            DebugBattleFixture::LayoutThree,
            DebugBattleFixture::LayoutFive,
        ] {
            apply_layout_review_statuses(&mut state, Some(fixture));
            for actor in &state.combatants {
                let badges = super::super::badge::badges(&actor.status_effects);
                assert_eq!(badges.len(), super::super::badge::MAX_BADGES);
                assert!(badges.iter().all(|badge| badge.icon_id().is_some() && badge.remaining_turns == Some(99)));
            }
        }
    }

    #[test]
    fn enemy_health_bars_fit_on_low_ground_backgrounds() {
        for ground in [
            full_enemy_ground(),
            GroundRect {
                x: 160,
                y: 340,
                width: 960,
                height: 80,
            },
            GroundRect {
                x: 120,
                y: 310,
                width: 910,
                height: 158,
            },
            GroundRect {
                x: 40,
                y: 351,
                width: 1200,
                height: 117,
            },
        ] {
            for sprite_size in [52.0_f32, 160.0, 288.0, 384.0] {
                for count in 1..=5 {
                    for index in 0..count {
                        let (_, top) = enemy_card_position(
                            ground,
                            sprite_size,
                            sprite_size.max(80.0),
                            count,
                            index,
                        );
                        // Status cards overlay the sprite; only its frame and health bar
                        // contribute to the bottom edge.
                        let bottom = top + sprite_size + 4.0 + 4.0 + 22.0;
                        assert!(
                            bottom <= ENEMY_AREA_HEIGHT,
                            "enemy health bar clipped: ground={ground:?}, sprite={sprite_size}, count={count}, index={index}, bottom={bottom}"
                        );
                    }
                }
            }
        }
    }

    fn layout_participant(size: EnemySize, scale: u32) -> crate::encounter::BattleParticipant {
        crate::encounter::BattleParticipant {
            side: BattleSide::Enemy,
            id: "orc".into(),
            name: "Orc".into(),
            class_id: String::new(),
            health: 1,
            max_health: 1,
            mana: 0,
            max_mana: 0,
            attack: 0,
            defense: 0,
            magic_resistance: 0,
            dexterity: 0,
            abilities: Vec::new(),
            status_effects: Vec::new(),
            accessory: None,
            wields_sword: false,
            row: crate::scenario_party::PartyRow::Front,
            boss: false,
            enemy_type: None,
            immunities: Vec::new(),
            behavior: None,
            experience_yield: 0,
            gold_yield: 0,
            enemy_size: Some(size),
            sprite_id: "orc".into(),
            sprite_scale_percent: scale,
            drops: None,
        }
    }

    #[test]
    fn a_v1_only_battle_keeps_the_legacy_layout() {
        let orc = layout_participant(EnemySize::Small, 270);
        for count in 1..=5 {
            for index in 0..count {
                let layout = enemy_card_layout(
                    full_enemy_ground(),
                    &orc,
                    &vec![false; count],
                    EnemyRows::All,
                    count,
                    index,
                );
                let (left, top) =
                    enemy_card_position(full_enemy_ground(), 140.0, 140.0, count, index);
                assert_eq!(
                    layout,
                    EnemyCardLayout {
                        sprite_size: 140.0,
                        card_width: 140.0,
                        left,
                        top,
                        z: 0,
                        bar_width: None,
                    }
                );
            }
        }
    }

    #[test]
    fn the_cast_shadow_starts_at_the_feet_and_falls_north_east() {
        for sprite_size in [256.0, 273.0, 300.0, 64.0] {
            let (parts, shift) = enemy_shadow_slant(sprite_size);
            // The nested UI transforms, applied about the tile centre as Bevy does.
            let place = |from_centre: Vec2| {
                shift
                    + Mat2::from_angle(parts.outer_rotation)
                        * Mat2::from_diagonal(parts.scale)
                        * Mat2::from_angle(parts.inner_rotation)
                        * from_centre
            };
            let feet = Vec2::new(0.0, sprite_size / 2.0 - ENEMY_SHADOW_FEET_INSET);
            assert!(place(feet).abs_diff_eq(feet, 1e-3), "{sprite_size}");
            let head = Vec2::new(0.0, -sprite_size / 2.0);
            let shadow_head = place(head);
            assert!(shadow_head.x > 0.0, "falls east: {shadow_head:?}");
            assert!(shadow_head.y < feet.y, "falls north: {shadow_head:?}");
            assert!(shadow_head.y > head.y, "lies flatter than the figure");
        }
    }

    #[test]
    fn back_mid_zones_keep_every_enemy_off_the_front_row() {
        let orc = layout_participant(EnemySize::Small, 270);
        for count in 1..=5 {
            for index in 0..count {
                let layout = enemy_card_layout(
                    full_enemy_ground(),
                    &orc,
                    &vec![true; count],
                    EnemyRows::BackMid,
                    count,
                    index,
                );
                assert!(layout.sprite_size < 300.0, "{count}/{index}");
                assert!(layout.z < 2, "{count}/{index}");
            }
        }
    }

    #[test]
    fn five_v2_enemies_take_the_approved_rows_on_the_mountain_pass() {
        let orc = layout_participant(EnemySize::Small, 270);
        let layouts = (0..5)
            .map(|index| {
                enemy_card_layout(
                    full_enemy_ground(),
                    &orc,
                    &[true; 5],
                    EnemyRows::All,
                    5,
                    index,
                )
            })
            .map(|layout| {
                (
                    layout.left + layout.card_width / 2.0,
                    layout.top + layout.sprite_size + 2.0,
                    layout.sprite_size,
                    layout.z,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            layouts,
            vec![
                (330.0, 352.0, 256.0, 0),
                (450.0, 436.0, 300.0, 2),
                (640.0, 385.0, 273.0, 1),
                (830.0, 436.0, 300.0, 2),
                (950.0, 352.0, 256.0, 0),
            ]
        );
    }

    #[test]
    fn a_mixed_battle_seats_v1_enemies_in_depth_rows_at_their_own_size() {
        let bruiser = layout_participant(EnemySize::Medium, 100);
        let layout = enemy_card_layout(
            full_enemy_ground(),
            &bruiser,
            &[false, true],
            EnemyRows::All,
            2,
            0,
        );
        assert_eq!(layout.sprite_size, 64.0);
        assert_eq!(layout.bar_width, None);
        assert_eq!(layout.left + layout.card_width / 2.0, 450.0);
        assert_eq!(layout.z, 2);
    }

    #[test]
    fn depth_layout_health_bars_fit_on_low_ground_backgrounds() {
        let orc = layout_participant(EnemySize::Small, 270);
        for ground in [
            full_enemy_ground(),
            GroundRect {
                x: 160,
                y: 340,
                width: 960,
                height: 80,
            },
            GroundRect {
                x: 120,
                y: 310,
                width: 910,
                height: 158,
            },
            GroundRect {
                x: 40,
                y: 351,
                width: 1200,
                height: 117,
            },
        ] {
            for count in 1..=5 {
                for index in 0..count {
                    let layout = enemy_card_layout(
                        ground,
                        &orc,
                        &vec![true; count],
                        EnemyRows::All,
                        count,
                        index,
                    );
                    let bottom = layout.top + layout.sprite_size + 4.0 + 4.0 + 22.0;
                    assert!(
                        bottom <= ENEMY_AREA_HEIGHT,
                        "v2 health bar clipped: ground={ground:?}, count={count}, index={index}, bottom={bottom}"
                    );
                    assert!(
                        layout.top >= 0.0,
                        "v2 head clipped: {ground:?} {count}/{index}"
                    );
                }
            }
        }
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
        assert_eq!(party_panel_width(), 778.0);
        assert_eq!(
            party_panel_width(),
            PARTY_SLOT_COUNT as f32 * PARTY_CARD_WIDTH
                + (PARTY_SLOT_COUNT - 1) as f32 * PARTY_CARD_GAP
        );
    }

    #[test]
    fn the_bottom_row_seats_every_panel_at_its_reserved_width() {
        // Panel row padding is 8 per side, with an 8px gap between each column.
        let available = crate::gameplay_canvas::LOGICAL_CANVAS_WIDTH as f32 - 8.0 * 2.0;
        let list = available - party_panel_width() - 8.0 - deck::GROUP_FRAME_WIDTH - 8.0;
        assert_eq!(list, deck::SPELL_FRAME_WIDTH);
    }

    #[test]
    fn the_spell_list_fits_the_longest_current_name_and_larger_mp_text() {
        // Lodestone Call is ~117px at the locked design's 18px name size.
        let usable =
            deck::SPELL_FRAME_WIDTH - 2.0 - 16.0 - 10.0 - 10.0 - ABILITY_ICON_SIZE - 12.0 - 50.0;
        assert!(usable >= 117.0 + 12.0);
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

        // Right is a second confirm on the command and sub-menus.
        assert!(confirm_pressed(&actions_from(&[Key::Confirm])));
        assert!(confirm_pressed(&actions_from(&[Key::MenuRight])));
        assert!(!confirm_pressed(&actions_from(&[Key::MenuLeft])));
        assert!(!confirm_pressed(&actions_from(&[])));

        // Left backs out of a sub-menu the way ESC does.
        assert!(cancel_pressed(&actions_from(&[Key::Back])));
        assert!(cancel_pressed(&actions_from(&[Key::MenuLeft])));
        assert!(!cancel_pressed(&actions_from(&[Key::MenuRight])));
        assert!(!cancel_pressed(&actions_from(&[])));
    }

    #[test]
    fn target_selection_cycles_horizontally_without_confirming() {
        use crate::input_record::NormalizedAction as Key;

        // Enemies stand in a row, so Left/Right walk it just as Up/Down do.
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
                spawn_status_badges(
                    root,
                    CombatantKey::party(0),
                    BadgeAnchor::FrameCorner,
                    &font,
                );
                spawn_status_badges(
                    root,
                    CombatantKey::enemy(0),
                    BadgeAnchor::SpriteTopCenter { sprite_size: 320.0 },
                    &font,
                );
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
            vec!["PSN 3".to_owned(), "SIL 2".to_owned(), String::new()],
            "both afflictions show, in the order they landed, and the spare slot stays blank"
        );
        assert_eq!(
            labels(CombatantKey::enemy(0), &mut app),
            vec!["ATK- 2".to_owned(), String::new(), String::new()],
            "an enemy weakened by Rune Bash wears the badge too, not just the party"
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

    /// The defect: every badge resolved, coloured and un-hid correctly, and
    /// not one of them reached the screen.
    ///
    /// The stack claimed `GlobalZIndex(15)` to sit above the portrait it overlaps, which reads as
    /// a local raise but is not one — Bevy lifts such a node clear of its parent and ranks it
    /// against the UI roots, so 15 put the pills *behind* the battle root at
    /// [`BATTLE_ROOT_Z`] and its opaque floor drew over them. The sibling tests here could not see
    /// it because their root has no `GlobalZIndex` for a child to sink beneath.
    #[test]
    fn a_badge_stack_is_layered_within_its_frame_and_not_behind_the_battle_screen() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let font = Handle::<Font>::default();
        let root = app
            .world_mut()
            .commands()
            .spawn((Node::default(), GlobalZIndex(BATTLE_ROOT_Z)))
            .with_children(|root| {
                root.spawn((Node::default(), BattlePartyCard(0)))
                    .with_children(|card| {
                        spawn_status_badges(
                            card,
                            CombatantKey::party(0),
                            BadgeAnchor::FrameCorner,
                            &font,
                        );
                    });
                root.spawn((Node::default(), BattleEnemyCard(0)))
                    .with_children(|card| {
                        spawn_status_badges(
                            card,
                            CombatantKey::enemy(0),
                            BadgeAnchor::SpriteTopCenter { sprite_size: 320.0 },
                            &font,
                        );
                    });
            })
            .id();
        app.update();

        assert_eq!(
            crate::battle::tests::hidden_behind_the_battle_root(app.world(), root),
            Vec::<i32>::new(),
            "a pill under the battle root's own layer is drawn beneath its opaque floor"
        );

        // It still has to outrank the portrait and meters it is pinned over, which is what the
        // global index was reaching for in the first place.
        let mut stacks = app.world_mut().query::<(&ZIndex, &Children)>();
        let badge_stacks = stacks
            .iter(app.world())
            .filter(|(_, children)| {
                children
                    .iter()
                    .any(|child| app.world().get::<BattleStatusBadge>(child).is_some())
            })
            .map(|(zindex, _)| zindex.0)
            .collect::<Vec<_>>();
        assert_eq!(
            badge_stacks,
            vec![BADGE_Z; 2],
            "both stacks keep a local raise over the card contents they share a line with"
        );
    }

    #[test]
    fn enemy_badges_scale_at_the_top_center_inside_the_sprite_tile() {
        for sprite_size in [52.0_f32, 160.0, 320.0, 384.0] {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            let font = Handle::<Font>::default();
            let tile = app
                .world_mut()
                .commands()
                .spawn(Node {
                    width: px(sprite_size),
                    height: px(sprite_size),
                    ..default()
                })
                .with_children(|tile| {
                    spawn_status_badges(
                        tile,
                        CombatantKey::enemy(0),
                        BadgeAnchor::SpriteTopCenter { sprite_size },
                        &font,
                    );
                })
                .id();
            app.update();

            let stack = app.world().get::<Children>(tile).unwrap()[0];
            let node = app.world().get::<Node>(stack).unwrap();
            assert_eq!(node.position_type, PositionType::Absolute);
            assert_eq!(node.flex_direction, FlexDirection::Row);
            assert_eq!(node.justify_content, JustifyContent::Center);
            assert_eq!(node.align_items, AlignItems::FlexStart);
            assert_eq!((node.top, node.left), (px(0), px(0)));
            assert_eq!((node.width, node.height), (percent(100), Val::Auto));
            let slots = app.world().get::<Children>(stack).unwrap();
            assert_eq!(slots.len(), super::super::badge::MAX_BADGES);
            for slot in slots.iter() {
                let badge = app.world().get::<BattleStatusBadge>(slot).unwrap();
                assert_eq!(badge.icon_size, (sprite_size * 0.2).floor().max(16.0));
                assert!(
                    badge.icon_size * slots.len() as f32 + (slots.len() - 1) as f32 <= sprite_size
                );
            }
        }
    }

    #[test]
    fn status_art_tracks_modifiers_fallbacks_ko_and_revival() {
        use crate::battle::status::{ActiveStatus, StatusEffect};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<ScenarioRoot>()
            .add_systems(Update, sync_status_badges);
        let mut inventory = ScenarioInventory::default();
        for id in [
            "poison",
            "attack_down",
            "attack_up",
            "ko",
            "haste",
            "slow",
            "regen",
        ] {
            inventory.status_images.insert(
                id.to_owned(),
                format!("media/images/icons/status/{id}.png")
                    .try_into()
                    .unwrap(),
            );
        }
        app.insert_resource(inventory);
        let font = Handle::<Font>::default();
        app.world_mut()
            .commands()
            .spawn(Node::default())
            .with_children(|root| {
                spawn_status_badges(
                    root,
                    CombatantKey::party(0),
                    BadgeAnchor::FrameCorner,
                    &font,
                );
                spawn_status_badges(
                    root,
                    CombatantKey::enemy(0),
                    BadgeAnchor::SpriteTopCenter { sprite_size: 320.0 },
                    &font,
                );
            });

        let mut party = crate::battle::tests::actor(BattleSide::Party, 0, 5, 20);
        party.status_effects = vec![
            ActiveStatus::persistent(StatusEffect::Poison),
            ActiveStatus::persistent(StatusEffect::Silence),
        ];
        let mut enemy = crate::battle::tests::actor(BattleSide::Enemy, 0, 4, 30);
        enemy.status_effects = vec![ActiveStatus::modifier(
            StatusEffect::AttackModifier,
            2,
            0.85,
        )];
        app.insert_resource(crate::battle::tests::state_with(vec![party, enemy]));
        app.update();

        let assert_art = |app: &mut App, key, expected: &str| {
            let world = app.world_mut();
            let mut query =
                world.query::<(&BattleStatusBadge, &Node, &ImageNode, &BackgroundColor)>();
            let (marker, node, image, background) = query
                .iter(world)
                .find(|(marker, ..)| marker.key == key && marker.slot == 0)
                .unwrap();
            assert_eq!(node.display, Display::Flex);
            let expected_size = if marker.key.side == BattleSide::Enemy {
                64.0
            } else {
                24.0
            };
            assert_eq!(
                (node.width, node.height),
                (px(expected_size), px(expected_size))
            );
            assert_eq!(image.color, Color::WHITE);
            assert_eq!(background.0, Color::srgba(0.0, 0.0, 0.0, 0.5));
            let path = world
                .resource::<AssetServer>()
                .get_path(image.image.id())
                .unwrap();
            assert_eq!(
                path.path().to_string_lossy(),
                format!(
                    "{}{expected}.png",
                    scenario_asset!("media/images/icons/status/")
                )
            );
        };
        assert_art(&mut app, CombatantKey::party(0), "poison");
        assert_art(&mut app, CombatantKey::enemy(0), "attack_down");
        for (marker, text) in app
            .world_mut()
            .query::<(&BattleStatusBadgeLabel, &Text)>()
            .iter(app.world())
        {
            assert_eq!(
                text.0,
                if marker.key == CombatantKey::party(0) && marker.slot == 1 {
                    "SIL"
                } else if marker.key == CombatantKey::enemy(0) && marker.slot == 0 {
                    "2"
                } else {
                    ""
                }
            );
        }

        {
            let mut state = app.world_mut().resource_mut::<BattleState>();
            state.combatants[0].health = 0;
            state.combatants[1].status_effects[0] =
                ActiveStatus::modifier(StatusEffect::AttackModifier, 2, 1.25);
        }
        app.update();
        assert_art(&mut app, CombatantKey::party(0), "ko");
        assert_art(&mut app, CombatantKey::enemy(0), "attack_up");

        for (status, expected) in [
            (ActiveStatus::speed(3, 1.5), "haste"),
            (ActiveStatus::speed(3, 0.5), "slow"),
            (ActiveStatus::regeneration(3, 0.05), "regen"),
        ] {
            for side in [0, 1] {
                let mut state = app.world_mut().resource_mut::<BattleState>();
                state.combatants[side].health = 10;
                state.combatants[side].status_effects = vec![status];
            }
            app.update();
            assert_art(&mut app, CombatantKey::party(0), expected);
            assert_art(&mut app, CombatantKey::enemy(0), expected);
            for (marker, text) in app
                .world_mut()
                .query::<(&BattleStatusBadgeLabel, &Text)>()
                .iter(app.world())
            {
                if marker.slot == 0 {
                    assert_eq!(text.0, "3");
                }
            }
        }

        {
            let mut state = app.world_mut().resource_mut::<BattleState>();
            state.combatants[0].health = 10;
            state.combatants[0].status_effects.clear();
            state.combatants[1].health = 0;
        }
        app.update();
        assert!(app.world_mut().query::<(&BattleStatusBadge, &Node, &ImageNode)>()
            .iter(app.world()).all(|(_, node, image)| node.display == Display::None && image.color == Color::NONE));
    }

    #[test]
    fn a_defeated_party_member_replaces_afflictions_with_ko() {
        use crate::battle::status::{ActiveStatus, StatusEffect};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, sync_status_badges);
        let font = Handle::<Font>::default();
        app.world_mut()
            .commands()
            .spawn(Node::default())
            .with_children(|root| {
                spawn_status_badges(
                    root,
                    CombatantKey::party(0),
                    BadgeAnchor::FrameCorner,
                    &font,
                )
            });

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

        for (marker, node) in app
            .world_mut()
            .query::<(&BattleStatusBadge, &Node)>()
            .iter(app.world())
        {
            assert_eq!(node.display == Display::Flex, marker.slot == 0);
        }
        let labels = app
            .world_mut()
            .query::<(&BattleStatusBadgeLabel, &Text)>()
            .iter(app.world())
            .filter(|(marker, _)| marker.slot == 0)
            .map(|(_, text)| text.0.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["KO"]);
    }

    #[test]
    fn the_battle_log_is_kept_for_the_field_menu_and_cleared_by_the_next_battle() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<LastBattleLog>()
            .add_systems(Update, sync_battle_message);
        let party = crate::battle::tests::actor(BattleSide::Party, 0, 5, 20);
        let mut state = crate::battle::tests::state_with(vec![party]);
        state.phase = BattlePhase::Resolve;
        state.message = "Elise attacks.".to_owned();
        app.world_mut().insert_resource(state);
        app.update();
        // An unchanged message is the same result still on screen, not a second one.
        app.world_mut().resource_mut::<BattleState>().set_changed();
        app.update();
        app.world_mut().resource_mut::<BattleState>().message =
            "Got away safely! Press Enter.".to_owned();
        app.update();
        // A prompt is an instruction, not something that happened.
        {
            let mut state = app.world_mut().resource_mut::<BattleState>();
            state.phase = BattlePhase::Command;
            state.message = "Choose a command.".to_owned();
        }
        app.update();

        let lines: Vec<_> = app
            .world()
            .resource::<LastBattleLog>()
            .lines()
            .iter()
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(lines, ["Elise attacks.", "Got away safely!"]);

        // The spoils follow the battle's own lines, once, however long the reward screen stays.
        app.world_mut().resource_mut::<BattleState>().rewards =
            Some(crate::battle::rewards::BattleRewards {
                total_experience: 12,
                gp_gained: 5,
                members: Vec::new(),
                loot: Vec::new(),
                shattered_cores: 0,
                boss_flag: None,
            });
        app.update();
        app.world_mut().resource_mut::<BattleState>().set_changed();
        app.update();
        let tail = app.world().resource::<LastBattleLog>().lines()[2..].to_vec();
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].text, "Won 12 EXP and 5 GP.");
        assert_eq!(tail[0].kind, LogKind::Reward);

        app.world_mut().remove_resource::<BattleState>();
        app.update();
        let party = crate::battle::tests::actor(BattleSide::Party, 0, 5, 20);
        app.world_mut()
            .insert_resource(crate::battle::tests::state_with(vec![party]));
        app.update();
        assert!(app.world().resource::<LastBattleLog>().is_empty());
    }

    #[test]
    fn battle_ui_sync_systems_have_disjoint_component_access() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<PlaySfx>()
            .add_message::<bevy::input::mouse::MouseWheel>()
            .init_resource::<FieldMenuCatalog>()
            .add_systems(
                Update,
                (
                    sync_party_cards,
                    sync_party_meters,
                    sync_status_badges,
                    sync_battle_commands,
                    deck::handle_pointer,
                    deck::sync_picker,
                    deck::sync_groups,
                    deck::sync_progress,
                    deck::sync_log_overlay,
                    sync_battle_message,
                    sync_enemy_cards,
                    sync_enemy_meters,
                    super::super::reward_modal::sync_reward_modal,
                    super::super::fx::route_battle_fx,
                    super::super::fx::animate_battle_fx,
                    super::super::fx::animate_battle_shake,
                    super::super::fx::animate_battle_dissolve,
                )
                    .chain(),
            );
        app.update();
    }
}
