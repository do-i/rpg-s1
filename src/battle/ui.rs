use bevy::{asset::LoadState, prelude::*};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    encounter::{BattleEntry, BattleSide, restore_pre_battle_context},
    game_state::GameState,
    scenario_balance::BalanceData,
    scenario_party::PartyRow,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    tsx_atlas_asset::TsxAtlasAsset,
    world_encounter::WorldEncounterRestore,
};

use super::{
    model::{
        BattleCommand, BattlePhase, BattleState, CombatantKey, FleeOutcome, TargetGroup,
        TargetSelector,
    },
    rules::{flee_chance, phase_after_flee_confirmation, roll_flee, wrap_index},
};

const BATTLE_IDLE_TILE: u32 = 8 * 9;
const COMMANDS: [BattleCommand; 4] = [
    BattleCommand::Attack,
    BattleCommand::Spell,
    BattleCommand::Item,
    BattleCommand::Run,
];

pub(crate) struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Battle), enter_battle)
            .add_systems(
                Update,
                (drive_battle_assets, handle_battle_input, sync_battle_ui)
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            )
            .add_systems(OnExit(AppState::Battle), cleanup_battle);
    }
}

#[derive(Component)]
struct BattleUi;

#[derive(Component)]
struct BattleEnemyImage(usize);

#[derive(Component)]
struct BattleEnemyLabel(usize);

#[derive(Component)]
struct BattlePartyText;

#[derive(Component)]
struct BattleCommandText;

#[derive(Component)]
struct BattleMessageText;

#[derive(Debug, Resource)]
struct BattleAssetState {
    atlases: Vec<Option<Handle<TsxAtlasAsset>>>,
}

fn enter_battle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    entry: Option<Res<BattleEntry>>,
) {
    let Some(entry) = entry else {
        return;
    };
    let state = BattleState::from_entry(&entry);
    let atlases = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .map(|participant| {
            ScenarioRelativePath::try_from(
                format!("assets/sprites/enemies/{}.tsx", participant.sprite_id).as_str(),
            )
            .ok()
            .map(|path| asset_server.load(root.resolve(&path)))
        })
        .collect::<Vec<_>>();
    let background = ScenarioRelativePath::try_from(entry.background_asset.as_str())
        .ok()
        .map(|path| asset_server.load(root.resolve(&path)))
        .unwrap_or_default();
    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("battle font path"),
        ),
    );
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(px(24)),
                ..default()
            },
            ImageNode::new(background).with_mode(NodeImageMode::Stretch),
            BackgroundColor(Color::srgb(0.03, 0.03, 0.08)),
            GlobalZIndex(200),
            Pickable::IGNORE,
            BattleUi,
        ))
        .with_children(|root_node| {
            root_node
                .spawn(Node {
                    width: percent(100),
                    height: px(390),
                    justify_content: JustifyContent::SpaceEvenly,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    for (index, participant) in entry
                        .participants
                        .iter()
                        .filter(|participant| participant.side == BattleSide::Enemy)
                        .enumerate()
                    {
                        row.spawn(Node {
                            width: px(180),
                            height: px(240),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            row_gap: px(8),
                            ..default()
                        })
                        .with_children(|card| {
                            card.spawn((
                                ImageNode::solid_color(Color::srgba(0.7, 0.1, 0.1, 0.7)),
                                Node {
                                    width: px(128),
                                    height: px(128),
                                    ..default()
                                },
                                BattleEnemyImage(index),
                            ));
                            card.spawn((
                                Text::new(participant.name.clone()),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(22.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                BattleEnemyLabel(index),
                            ));
                        });
                    }
                });
            root_node
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(285),
                        flex_direction: FlexDirection::Row,
                        padding: UiRect::all(px(18)),
                        column_gap: px(28),
                        border_radius: BorderRadius::all(px(12)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.015, 0.02, 0.09, 0.92)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(19.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(235, 225, 190)),
                        Node {
                            width: percent(48),
                            ..default()
                        },
                        BattlePartyText,
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Node {
                            width: percent(20),
                            ..default()
                        },
                        BattleCommandText,
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.into(),
                            font_size: FontSize::Px(19.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(117, 220, 214)),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        BattleMessageText,
                    ));
                });
        });
    commands.insert_resource(BattleAssetState { atlases });
    commands.insert_resource(state);
}

fn drive_battle_assets(
    asset_server: Res<AssetServer>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    assets: Option<Res<BattleAssetState>>,
    mut images: Query<(&BattleEnemyImage, &mut ImageNode)>,
) {
    let Some(assets) = assets else { return };
    for (marker, mut image) in &mut images {
        let Some(Some(handle)) = assets.atlases.get(marker.0) else {
            continue;
        };
        if matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)) {
            continue;
        }
        let Some(atlas) = atlases.get(handle) else {
            continue;
        };
        if let Ok(sprite) = atlas.sprite_for_tile(BATTLE_IDLE_TILE) {
            image.image = sprite.image;
            image.texture_atlas = sprite.texture_atlas;
            image.color = Color::WHITE;
        }
    }
}

fn handle_battle_input(
    mut commands: Commands,
    actions: Res<ActionState>,
    balances: Res<Assets<BalanceData>>,
    mut game: Option<ResMut<GameState>>,
    entry: Option<Res<BattleEntry>>,
    state: Option<ResMut<BattleState>>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
) {
    let (Some(entry), Some(mut state), Some(mut game)) = (entry, state, game.take()) else {
        return;
    };
    match state.phase {
        BattlePhase::Start => state.begin_active_turn(game.rng_mut()),
        BattlePhase::Advance => state.advance(game.rng_mut()),
        BattlePhase::Command => {
            if actions.just_pressed(AppAction::Back) {
                attempt_flee(&mut state, &balances, &mut game);
                return;
            }
            if let Some(movement) = actions.menu_navigation() {
                state.command_index = wrap_index(state.command_index, movement, COMMANDS.len());
            }
            if actions.just_pressed(AppAction::Confirm) {
                let command = COMMANDS[state.command_index];
                if !state.command_available(command) {
                    state.message = match command {
                        BattleCommand::Spell | BattleCommand::Item => {
                            format!("{} effects unlock in Milestone 10.", command.label())
                        }
                        BattleCommand::Run => "Can't escape from a boss!".to_owned(),
                        BattleCommand::Attack => "That action is unavailable.".to_owned(),
                    };
                    return;
                }
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
                        attempt_flee(&mut state, &balances, &mut game);
                    }
                    BattleCommand::Spell | BattleCommand::Item => {}
                }
            }
        }
        BattlePhase::Target => {
            if actions.just_pressed(AppAction::Back) {
                state.target = None;
                state.phase = BattlePhase::Command;
                state.message = "Action cancelled.".to_owned();
            } else {
                if let Some(movement) = actions.menu_navigation()
                    && let Some(target) = state.target.as_mut()
                {
                    target.navigate(movement);
                }
                if actions.just_pressed(AppAction::Confirm) {
                    let attacker = state.active_key();
                    let target = state.target.as_ref().map(TargetSelector::selected);
                    state.target = None;
                    if let (Some(attacker), Some(target)) = (attacker, target) {
                        state.resolve_physical(attacker, target, game.rng_mut());
                    }
                }
            }
        }
        BattlePhase::Resolve if actions.just_pressed(AppAction::Confirm) => state.assess_result(),
        BattlePhase::Victory if actions.just_pressed(AppAction::Confirm) => {
            apply_victory(&mut commands, &mut game, &entry, &state);
            transitions.write(AppStateTransitionRequest::new(AppState::World));
        }
        BattlePhase::Defeat => {
            transitions.write(AppStateTransitionRequest::new(AppState::GameOver));
        }
        BattlePhase::Flee if actions.just_pressed(AppAction::Confirm) => {
            let outcome = state.flee_outcome.unwrap_or(FleeOutcome::Failed);
            if phase_after_flee_confirmation(outcome).is_none() {
                restore_world(&mut commands, &mut game, &entry);
                transitions.write(AppStateTransitionRequest::new(AppState::World));
            } else {
                state.phase = phase_after_flee_confirmation(outcome)
                    .expect("failed and blocked flee consume the active turn");
            }
        }
        _ => {}
    }
}

fn attempt_flee(state: &mut BattleState, balances: &Assets<BalanceData>, game: &mut GameState) {
    let Some(balance) = balances.iter().next().map(|(_, value)| value) else {
        state.message = "Battle balance is still loading.".to_owned();
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
    state.transcript.push(format!("FLEE {outcome:?}"));
}

fn apply_victory(
    commands: &mut Commands,
    game: &mut GameState,
    entry: &BattleEntry,
    state: &BattleState,
) {
    for actor in state
        .combatants
        .iter()
        .filter(|actor| actor.key.side == BattleSide::Party)
    {
        if let Some(member) = game.party_mut().member_mut(&actor.id) {
            member.apply_damage(member.health().saturating_sub(actor.health));
            member.spend_mana(member.mana().saturating_sub(actor.mana));
        }
    }
    if let Some(flag) = entry.boss_completion_flag.as_deref()
        && !flag.is_empty()
    {
        game.flags_mut().set(flag);
    }
    restore_world(commands, game, entry);
}

fn restore_world(commands: &mut Commands, game: &mut GameState, entry: &BattleEntry) {
    if restore_pre_battle_context(game, &entry.return_context).is_ok() {
        commands.insert_resource(WorldEncounterRestore {
            map_id: entry.return_context.map_id.clone(),
            enemies: entry.return_context.world_enemies.clone(),
        });
    }
}

#[expect(
    clippy::type_complexity,
    reason = "disjoint UI marker filters prevent mutable Text query overlap"
)]
fn sync_battle_ui(
    state: Option<Res<BattleState>>,
    mut party_text: Query<
        &mut Text,
        (
            With<BattlePartyText>,
            Without<BattleCommandText>,
            Without<BattleEnemyLabel>,
        ),
    >,
    mut command_text: Query<
        &mut Text,
        (
            With<BattleCommandText>,
            Without<BattlePartyText>,
            Without<BattleMessageText>,
            Without<BattleEnemyLabel>,
        ),
    >,
    mut message_text: Query<
        &mut Text,
        (
            With<BattleMessageText>,
            Without<BattlePartyText>,
            Without<BattleCommandText>,
            Without<BattleEnemyLabel>,
        ),
    >,
    mut enemy_images: Query<(&BattleEnemyImage, &mut ImageNode)>,
    mut enemy_labels: Query<
        (&BattleEnemyLabel, &mut Text, &mut TextColor),
        (
            Without<BattlePartyText>,
            Without<BattleCommandText>,
            Without<BattleMessageText>,
        ),
    >,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    let active = state.active_key();
    if let Ok(mut text) = party_text.single_mut() {
        text.0 = state
            .combatants
            .iter()
            .filter(|actor| actor.key.side == BattleSide::Party)
            .map(|actor| {
                format!(
                    "{}{}\n  HP {}/{}  MP {}/{}  {}{}",
                    if active == Some(actor.key) {
                        "> "
                    } else {
                        "  "
                    },
                    actor.name,
                    actor.health,
                    actor.max_health,
                    actor.mana,
                    actor.max_mana,
                    match actor.row {
                        PartyRow::Front => "Front",
                        PartyRow::Back => "Back",
                    },
                    if actor.is_alive() { "" } else { "  KO" },
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
    }
    if let Ok(mut text) = command_text.single_mut() {
        text.0 = COMMANDS
            .iter()
            .enumerate()
            .map(|(index, command)| {
                let selected = state.phase == BattlePhase::Command && index == state.command_index;
                let available = state.command_available(*command);
                format!(
                    "{}{}{}",
                    if selected { "> " } else { "  " },
                    command.label(),
                    if available { "" } else { " [--]" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
    }
    if let Ok(mut text) = message_text.single_mut() {
        let target = state
            .target
            .as_ref()
            .and_then(|selector| state.actor(selector.selected()))
            .map(|actor| format!("\n\nTarget: {}", actor.name))
            .unwrap_or_default();
        text.0 = format!("{:?}\n\n{}{}", state.phase, state.message, target);
    }
    let selected_target = state.target.as_ref().map(TargetSelector::selected);
    for (marker, mut image) in &mut enemy_images {
        let key = CombatantKey::enemy(marker.0);
        let Some(enemy) = state.actor(key) else {
            continue;
        };
        image.color = if !enemy.is_alive() {
            Color::srgba(0.3, 0.3, 0.3, 0.45)
        } else if selected_target == Some(key) {
            Color::srgb(1.0, 0.85, 0.35)
        } else {
            Color::WHITE
        };
    }
    for (marker, mut text, mut color) in &mut enemy_labels {
        let key = CombatantKey::enemy(marker.0);
        let Some(enemy) = state.actor(key) else {
            continue;
        };
        text.0 = format!(
            "{}  HP {}/{}{}",
            enemy.name,
            enemy.health,
            enemy.max_health,
            if enemy.is_alive() { "" } else { "  KO" }
        );
        color.0 = if selected_target == Some(key) {
            Color::srgb_u8(255, 220, 90)
        } else {
            Color::WHITE
        };
    }
}

fn cleanup_battle(mut commands: Commands, entities: Query<Entity, With<BattleUi>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<BattleState>();
    commands.remove_resource::<BattleAssetState>();
}
