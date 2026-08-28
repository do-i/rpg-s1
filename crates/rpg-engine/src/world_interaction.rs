//! Facing-aware World interaction, field dialogue loading, effects, and overlay rendering.

use std::{error::Error, fmt};

use bevy::{
    asset::{AssetApp, AssetLoader, LoadContext, LoadState, io::Reader},
    audio::Volume,
    prelude::*,
    reflect::TypePath,
};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::AppState,
    field_menu::FieldMenuState,
    game_state::GameState,
    runtime_member::RuntimeMember,
    scenario_audio::{SFX_INDEX_PATH, SfxIndex},
    scenario_balance::BalanceData,
    scenario_dialogue::{DialogueActions, DialogueDocument},
    scenario_inventory::ScenarioInventory,
    scenario_map::MagicCoreSize,
    scenario_party::PartyCatalog,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::{CardinalDirection, Position},
    scenario_yaml::{self, ScenarioYamlError},
    service_ui::{ServiceRequest, ServiceUiState},
    ui_theme::UiTheme,
    world_actor::WorldNpc,
    world_dialogue::{DialogueEvent, DialoguePhase, DialogueSession, apply_flag_actions},
    world_object::{WorldItemBox, WorldSign},
    world_player::{WorldPlayer, WorldPlayerMotion},
    world_transition::WorldTransition,
};

pub(crate) struct WorldInteractionPlugin;

impl Plugin for WorldInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<DialogueDocument>()
            .init_asset_loader::<DialogueDocumentAssetLoader>()
            .init_resource::<WorldInteractionState>()
            .add_systems(OnEnter(AppState::World), begin_world_interactions)
            .add_systems(
                Update,
                (
                    request_npc_dialogue,
                    resolve_dialogue_request,
                    drive_dialogue_session,
                    play_interaction_sfx,
                    sync_dialogue_overlay,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), cleanup_world_interactions);
    }
}

#[derive(Debug, Default, Resource)]
pub(crate) struct WorldInteractionState {
    request: Option<DialogueRequest>,
    session: Option<DialogueSession>,
    party: Option<Handle<PartyCatalog>>,
    balance: Option<Handle<BalanceData>>,
    sfx_index: Option<Handle<SfxIndex>>,
    pending_sounds: Vec<InteractionSound>,
    failure: Option<String>,
    sfx_failure: Option<String>,
    suppress_confirm: bool,
}

impl WorldInteractionState {
    pub(crate) const fn input_locked(&self) -> bool {
        self.request.is_some() || self.session.is_some()
    }

    pub(crate) fn session(&self) -> Option<&DialogueSession> {
        self.session.as_ref()
    }

    /// A state with a dialogue open, for tests that need the World input-locked.
    #[cfg(test)]
    pub(crate) fn dialogue_open_for_tests() -> Self {
        Self {
            request: Some(DialogueRequest {
                id: "test_dialogue".to_owned(),
                speaker: None,
                handle: Handle::default(),
            }),
            ..default()
        }
    }
}

#[derive(Debug)]
struct DialogueRequest {
    id: String,
    speaker: Option<String>,
    handle: Handle<DialogueDocument>,
}

#[derive(Component)]
struct WorldDialogueRoot;

#[derive(Component)]
struct WorldDialogueSpeaker;

#[derive(Component)]
struct WorldDialogueBody;

#[derive(Component)]
struct WorldDialogueChoices;

#[derive(Component)]
struct WorldDialogueHint;

fn begin_world_interactions(
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    mut state: ResMut<WorldInteractionState>,
) {
    let (Some(party), Some(balance)) = (inventory.party.as_ref(), inventory.balance.as_ref())
    else {
        *state = WorldInteractionState::default();
        return;
    };
    *state = WorldInteractionState {
        party: Some(asset_server.load(scenario_root.resolve(party))),
        balance: Some(asset_server.load(scenario_root.resolve(balance))),
        sfx_index: Some(asset_server.load(scenario_root.resolve(
            &ScenarioRelativePath::try_from(SFX_INDEX_PATH).expect("canonical SFX index path"),
        ))),
        ..default()
    };
}

#[expect(
    clippy::too_many_arguments,
    reason = "interaction arbitration intentionally queries each distinct World target class"
)]
fn request_npc_dialogue(
    actions: Res<ActionState>,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    transition: Res<WorldTransition>,
    field_menu: Res<FieldMenuState>,
    service: Res<ServiceUiState>,
    game: Option<ResMut<GameState>>,
    npcs: Query<&WorldNpc>,
    signs: Query<&WorldSign>,
    boxes: Query<&WorldItemBox>,
    players: Query<&WorldPlayerMotion, With<WorldPlayer>>,
    mut state: ResMut<WorldInteractionState>,
) {
    if state.input_locked()
        || field_menu.input_locked()
        || service.input_locked()
        || transition.input_locked()
        || !actions.just_pressed(AppAction::Confirm)
    {
        return;
    }
    let Some(mut game) = game else {
        return;
    };
    let player = game.map().position();
    let facing = game.map().facing();
    let player_pixels = players
        .single()
        .map(|motion| motion.top_left())
        .unwrap_or_else(|_| WorldPlayerMotion::from_tile(player).top_left());
    let npc = select_npc(player_pixels, facing, npcs.iter()).map(|target| {
        (
            distance_squared(player, target.tile_position()),
            0_u8,
            InteractionTarget::Dialogue {
                id: target.dialogue_id().to_owned(),
                speaker: Some(target.name().to_owned()),
            },
        )
    });
    let sign = signs
        .iter()
        .filter(|sign| {
            is_in_facing_direction(player, sign.tile_position(), facing)
                && within_range(player, sign.tile_position(), 1.5)
        })
        .min_by_key(|sign| (distance_squared(player, sign.tile_position()), sign.id()))
        .map(|sign| {
            (
                distance_squared(player, sign.tile_position()),
                1_u8,
                InteractionTarget::Dialogue {
                    id: sign.dialogue_id().to_owned(),
                    speaker: None,
                },
            )
        });
    let item_box = boxes
        .iter()
        .filter(|item_box| {
            is_in_facing_direction(player, item_box.tile_position(), facing)
                && within_range(player, item_box.tile_position(), 1.5)
        })
        .min_by_key(|item_box| distance_squared(player, item_box.tile_position()))
        .map(|item_box| {
            (
                distance_squared(player, item_box.tile_position()),
                2_u8,
                InteractionTarget::Box(item_box),
            )
        });
    let Some((_, _, target)) = [npc, sign, item_box]
        .into_iter()
        .flatten()
        .min_by_key(|(distance, priority, _)| (*distance, *priority))
    else {
        state.pending_sounds.push(InteractionSound::Blocked);
        return;
    };
    if let InteractionTarget::Box(item_box) = target {
        let outcome = open_item_box(item_box, &mut game);
        state.pending_sounds.push(if outcome.opened {
            InteractionSound::Box
        } else {
            InteractionSound::Blocked
        });
        state.session = Some(DialogueSession::message(
            format!("box_{}", item_box.id()),
            Some("Treasure".to_owned()),
            vec![outcome.message],
        ));
        state.suppress_confirm = true;
        state.failure = None;
        return;
    }
    let InteractionTarget::Dialogue { id, speaker } = target else {
        unreachable!();
    };
    let logical = format!("data/dialogue/{id}.yaml");
    let Ok(logical) = ScenarioRelativePath::try_from(logical.as_str()) else {
        state.pending_sounds.push(InteractionSound::Blocked);
        state.failure = Some(format!("dialogue id `{id}` cannot select a scenario file"));
        return;
    };
    state.request = Some(DialogueRequest {
        id,
        speaker,
        handle: asset_server.load(scenario_root.resolve(&logical)),
    });
    // The interaction sound plays once `resolve_dialogue_request` confirms a session actually
    // opens, not here. The pinned Python engine only creates its dialogue overlay after
    // `DialogueEngine.resolve` returns a match (`engine/world/world_map_scene.py::_try_interact`),
    // so a target whose dialogue document is missing (e.g. the two painted `zone_03_marshland`
    // sign tiles — no `sign_zone_03_marshland` document exists in either repository, see
    // `docs/adr/0007-inherited-scenario-data-debt.md`) or whose entries don't match the current
    // flags never makes a sound in the original. Queuing the sound eagerly here, before the async
    // load even resolves, used to make a missing sign click and then show nothing — this player
    // audible difference is what ADR 0007 flagged as needing a decision at W12.3 acceptance.
    state.failure = None;
}

enum InteractionTarget<'a> {
    Dialogue { id: String, speaker: Option<String> },
    Box(&'a WorldItemBox),
}

fn select_npc<'a>(
    player_pixels: Vec2,
    facing: CardinalDirection,
    npcs: impl Iterator<Item = &'a WorldNpc>,
) -> Option<&'a WorldNpc> {
    npcs.filter(|npc| {
        let target = npc.source_pixel_position();
        !npc.map_id().is_empty()
            && is_in_facing_direction_pixels(player_pixels, target, facing)
            && within_pixel_range(player_pixels, target, npc.interaction_range_pixels())
    })
    .min_by(|left, right| {
        player_pixels
            .distance_squared(left.source_pixel_position())
            .total_cmp(&player_pixels.distance_squared(right.source_pixel_position()))
            .then_with(|| left.name().cmp(right.name()))
    })
}

fn is_in_facing_direction_pixels(player: Vec2, target: Vec2, facing: CardinalDirection) -> bool {
    let delta = target - player;
    match facing {
        CardinalDirection::Up => delta.y < 0.0 && delta.y.abs() >= delta.x.abs(),
        CardinalDirection::Down => delta.y > 0.0 && delta.y.abs() >= delta.x.abs(),
        CardinalDirection::Left => delta.x < 0.0 && delta.x.abs() >= delta.y.abs(),
        CardinalDirection::Right => delta.x > 0.0 && delta.x.abs() >= delta.y.abs(),
    }
}

fn within_pixel_range(player: Vec2, target: Vec2, range: f32) -> bool {
    let delta = (target - player).abs();
    delta.x <= range && delta.y <= range
}

fn is_in_facing_direction(player: Position, target: Position, facing: CardinalDirection) -> bool {
    let dx = target.x - player.x;
    let dy = target.y - player.y;
    match facing {
        CardinalDirection::Up => dy < 0 && dy.abs() >= dx.abs(),
        CardinalDirection::Down => dy > 0 && dy.abs() >= dx.abs(),
        CardinalDirection::Left => dx < 0 && dx.abs() >= dy.abs(),
        CardinalDirection::Right => dx > 0 && dx.abs() >= dy.abs(),
    }
}

fn within_range(player: Position, target: Position, range: f32) -> bool {
    (target.x - player.x).abs() as f32 <= range && (target.y - player.y).abs() as f32 <= range
}

fn distance_squared(left: Position, right: Position) -> i64 {
    let dx = i64::from(right.x) - i64::from(left.x);
    let dy = i64::from(right.y) - i64::from(left.y);
    dx * dx + dy * dy
}

#[derive(Debug, Eq, PartialEq)]
struct ItemBoxOutcome {
    message: String,
    opened: bool,
}

fn open_item_box(item_box: &WorldItemBox, game: &mut GameState) -> ItemBoxOutcome {
    let key = item_box.key();
    if game.opened_boxes().contains(&key) {
        return ItemBoxOutcome {
            message: "This treasure box is already open.".to_owned(),
            opened: false,
        };
    }

    let mut grants = Vec::new();
    let batch = (!item_box.loot().items.is_empty() || !item_box.loot().magic_cores.is_empty())
        .then(|| game.repository_mut().start_loot_batch());
    for item in &item_box.loot().items {
        let _outcome = game
            .repository_mut()
            .add_item_in_batch(
                &item.id,
                item.qty.get(),
                batch.expect("nonempty item-box loot started a batch"),
            )
            .expect("validated item-box loot uses a current batch");
        grants.push(format!("{} ×{}", item.id, item.qty));
    }
    for core in &item_box.loot().magic_cores {
        let size = match core.size {
            MagicCoreSize::Xs => "xs",
            MagicCoreSize::S => "s",
            MagicCoreSize::M => "m",
            MagicCoreSize::L => "l",
            MagicCoreSize::Xl => "xl",
        };
        let id = format!("mc_{size}");
        let _outcome = game
            .repository_mut()
            .add_item_in_batch(
                &id,
                core.qty.get(),
                batch.expect("nonempty item-box loot started a batch"),
            )
            .expect("validated item-box core uses a current batch");
        grants.push(format!("{id} ×{}", core.qty));
    }
    game.opened_boxes_mut().record(key);
    let message = if grants.is_empty() {
        "The treasure box was empty.".to_owned()
    } else {
        format!("Found {}.", grants.join(", "))
    };
    ItemBoxOutcome {
        message,
        opened: true,
    }
}

fn resolve_dialogue_request(
    asset_server: Res<AssetServer>,
    documents: Res<Assets<DialogueDocument>>,
    game: Option<Res<GameState>>,
    mut state: ResMut<WorldInteractionState>,
) {
    let Some(request) = state.request.as_ref() else {
        return;
    };
    if matches!(
        asset_server.load_state(request.handle.id()),
        LoadState::Failed(_)
    ) {
        state.failure = Some(format!("dialogue `{}` failed to load", request.id));
        state.request = None;
        return;
    }
    let Some(document) = documents.get(&request.handle) else {
        return;
    };
    let Some(game) = game else {
        return;
    };
    if document.effective_id(&request.id) != request.id {
        state.failure = Some(format!("dialogue `{}` has a mismatched id", request.id));
        state.request = None;
        return;
    }
    let DialogueDocument::Entries(dialogue) = document else {
        state.failure = Some(format!("dialogue `{}` is not a field dialogue", request.id));
        state.request = None;
        return;
    };
    match DialogueSession::resolve(
        request.id.clone(),
        request.speaker.clone(),
        dialogue.clone(),
        game.flags(),
    ) {
        Ok(Some(session)) => {
            state.session = Some(session);
            state.suppress_confirm = true;
            state.request = None;
            state.failure = None;
            state.pending_sounds.push(InteractionSound::Dialogue);
        }
        Ok(None) => {
            state.request = None;
        }
        Err(error) => {
            state.failure = Some(format!("dialogue `{}` is invalid: {error}", request.id));
            state.request = None;
        }
    }
}

fn drive_dialogue_session(
    time: Res<Time>,
    actions: Res<ActionState>,
    party_assets: Res<Assets<PartyCatalog>>,
    balance_assets: Res<Assets<BalanceData>>,
    game: Option<ResMut<GameState>>,
    mut service: ResMut<ServiceUiState>,
    mut state: ResMut<WorldInteractionState>,
) {
    let Some(mut game) = game else {
        return;
    };
    if state.suppress_confirm {
        state.suppress_confirm = false;
        return;
    }
    let Some(session) = state.session.as_mut() else {
        return;
    };
    session.tick(time.delta_secs());
    if actions.just_pressed(AppAction::Back) {
        session.cancel();
        state.session = None;
        state.pending_sounds.push(InteractionSound::Cancel);
        return;
    }
    if let Some(delta) = actions.menu_navigation() {
        session.move_choice(if delta < 0 { -1 } else { 1 });
    }
    if !actions.just_pressed(AppAction::Confirm) {
        return;
    }
    let event = session.confirm(game.flags());
    state
        .pending_sounds
        .push(if event == DialogueEvent::Blocked {
            InteractionSound::Blocked
        } else {
            InteractionSound::Confirm
        });
    if let DialogueEvent::Apply(completions) = event {
        let party = state
            .party
            .as_ref()
            .and_then(|handle| party_assets.get(handle));
        let balance = state
            .balance
            .as_ref()
            .and_then(|handle| balance_assets.get(handle));
        for completion in completions {
            if let Some(request) = ServiceRequest::from_dialogue(&completion) {
                service.open(request);
            }
            if let Err(error) = apply_dialogue_actions(&completion, &mut game, party, balance) {
                state.failure = Some(error.to_string());
            }
        }
    }
    if state
        .session
        .as_ref()
        .is_some_and(|session| session.phase() == DialoguePhase::Closed)
    {
        state.session = None;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InteractionSound {
    Confirm,
    Blocked,
    Dialogue,
    Box,
    Cancel,
}

impl InteractionSound {
    const fn source_key(self) -> &'static str {
        match self {
            Self::Confirm | Self::Dialogue => "confirm",
            Self::Blocked => "denied",
            Self::Box => "use_item",
            Self::Cancel => "cancel",
        }
    }
}

#[derive(Component, Debug, Eq, PartialEq)]
struct WorldInteractionSfx {
    logical_event: InteractionSound,
    source_key: &'static str,
    asset_path: String,
}

fn play_interaction_sfx(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    indexes: Res<Assets<SfxIndex>>,
    mut state: ResMut<WorldInteractionState>,
) {
    if state.pending_sounds.is_empty() {
        return;
    }
    let Some(handle) = state.sfx_index.as_ref() else {
        state.sfx_failure = Some("scenario SFX index was not requested".to_owned());
        return;
    };
    if matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)) {
        state.sfx_failure = Some("scenario SFX index failed to load".to_owned());
        state.pending_sounds.clear();
        return;
    }
    let Some(index) = indexes.get(handle) else {
        return;
    };
    let pending_sounds = std::mem::take(&mut state.pending_sounds);
    for logical_event in pending_sounds {
        let source_key = logical_event.source_key();
        let Some(asset_path) = index.resolve_key(&scenario_root, source_key) else {
            state.sfx_failure = Some(format!(
                "interaction SFX `{source_key}` is missing from the scenario index"
            ));
            continue;
        };
        commands.spawn((
            AudioPlayer::new(asset_server.load(asset_path.clone())),
            PlaybackSettings {
                volume: Volume::Linear(0.6),
                ..PlaybackSettings::DESPAWN
            },
            WorldInteractionSfx {
                logical_event,
                source_key,
                asset_path,
            },
        ));
        state.sfx_failure = None;
    }
}

fn apply_dialogue_actions(
    actions: &DialogueActions,
    game: &mut GameState,
    party: Option<&PartyCatalog>,
    balance: Option<&BalanceData>,
) -> Result<(), DialogueActionError> {
    apply_flag_actions(actions, game.flags_mut());
    let gift_batch =
        (!actions.give_items.is_empty()).then(|| game.repository_mut().start_loot_batch());
    for gift in &actions.give_items {
        let _outcome = game
            .repository_mut()
            .add_item_in_batch(
                &gift.id,
                gift.qty.get(),
                gift_batch.expect("nonempty dialogue gift list started a batch"),
            )
            .map_err(|error| DialogueActionError::Item(error.to_string()))?;
    }
    let Some(member_id) = actions.join_party.as_deref() else {
        return Ok(());
    };
    if game.party().contains(member_id) {
        return Ok(());
    }
    let party = party.ok_or(DialogueActionError::PartyCatalogUnavailable)?;
    let balance = balance.ok_or(DialogueActionError::BalanceUnavailable)?;
    let source = party
        .party
        .iter()
        .find(|member| member.data().id == member_id)
        .ok_or_else(|| DialogueActionError::UnknownPartyMember(member_id.to_owned()))?;
    let runtime = RuntimeMember::try_from_catalog(source, &balance.progression)
        .map_err(|error| DialogueActionError::PartyMember(error.to_string()))?;
    game.party_mut()
        .try_add(runtime)
        .map_err(|error| DialogueActionError::PartyMember(error.to_string()))?;
    Ok(())
}

#[derive(Debug)]
enum DialogueActionError {
    Item(String),
    PartyCatalogUnavailable,
    BalanceUnavailable,
    UnknownPartyMember(String),
    PartyMember(String),
}

impl fmt::Display for DialogueActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Item(error) => write!(formatter, "dialogue item grant failed: {error}"),
            Self::PartyCatalogUnavailable => formatter.write_str("party catalog is unavailable"),
            Self::BalanceUnavailable => formatter.write_str("balance data is unavailable"),
            Self::UnknownPartyMember(id) => {
                write!(formatter, "dialogue names unknown party member `{id}`")
            }
            Self::PartyMember(error) => write!(formatter, "dialogue party join failed: {error}"),
        }
    }
}

impl Error for DialogueActionError {}

#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "the dialogue overlay updates disjoint Bevy Text roles in one synchronized pass"
)]
fn sync_dialogue_overlay(
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
            Without<WorldDialogueHint>,
        ),
    >,
    mut bodies: Query<
        &mut Text,
        (
            With<WorldDialogueBody>,
            Without<WorldDialogueSpeaker>,
            Without<WorldDialogueChoices>,
            Without<WorldDialogueHint>,
        ),
    >,
    mut choices: Query<
        &mut Text,
        (
            With<WorldDialogueChoices>,
            Without<WorldDialogueSpeaker>,
            Without<WorldDialogueBody>,
            Without<WorldDialogueHint>,
        ),
    >,
    mut hints: Query<
        &mut Text,
        (
            With<WorldDialogueHint>,
            Without<WorldDialogueSpeaker>,
            Without<WorldDialogueBody>,
            Without<WorldDialogueChoices>,
        ),
    >,
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
    for (_, mut name) in &mut roots {
        name.set(format!("World dialogue: {}", session.id()));
    }
    if let Ok(mut speaker) = speakers.single_mut() {
        speaker.0 = session.speaker().unwrap_or_default().to_owned();
    }
    if let Ok(mut body) = bodies.single_mut() {
        body.0 = session.visible_text();
    }
    if let Ok(mut choice_text) = choices.single_mut() {
        choice_text.0 = session
            .choices()
            .iter()
            .enumerate()
            .map(|(index, choice)| {
                let cursor = if index == session.selected_choice() {
                    ">"
                } else {
                    " "
                };
                let disabled = if choice.enabled() { "" } else { " [locked]" };
                format!("{cursor} {}{disabled}", choice.text())
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    if let Ok(mut hint) = hints.single_mut() {
        hint.0 = match session.phase() {
            DialoguePhase::Typing => "ENTER · reveal     ESC · close",
            DialoguePhase::Ready => ">  ENTER · continue     ESC · close",
            DialoguePhase::Choosing => "UP/DOWN · choose     ENTER · select     ESC · close",
            DialoguePhase::Closed => "",
        }
        .to_owned();
    }
}

fn spawn_dialogue_overlay(commands: &mut Commands, theme: &UiTheme, font: Handle<Font>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(20),
                bottom: px(20),
                width: px(920),
                height: px(180),
                padding: UiRect::all(px(16)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            },
            BackgroundColor(Color::srgba(0.047, 0.047, 0.118, 0.94)),
            BorderColor::all(theme.name_entry_border_color),
            GlobalZIndex(5_000),
            Pickable::IGNORE,
            Name::new("World dialogue"),
            WorldDialogueRoot,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(235, 210, 140)),
                WorldDialogueSpeaker,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(22.0),
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
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(theme.name_entry_input_color),
                WorldDialogueChoices,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(theme.name_entry_hint_color),
                WorldDialogueHint,
            ));
        });
}

fn cleanup_world_interactions(
    mut commands: Commands,
    roots: Query<Entity, With<WorldDialogueRoot>>,
    mut state: ResMut<WorldInteractionState>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    *state = WorldInteractionState::default();
}

#[derive(Default, TypePath)]
struct DialogueDocumentAssetLoader;

impl AssetLoader for DialogueDocumentAssetLoader {
    type Asset = DialogueDocument;
    type Settings = ();
    type Error = DialogueDocumentAssetError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(DialogueDocumentAssetError::Io)?;
        let document = std::str::from_utf8(&bytes).map_err(DialogueDocumentAssetError::Utf8)?;
        scenario_yaml::from_str(document).map_err(DialogueDocumentAssetError::Yaml)
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

#[derive(Debug)]
enum DialogueDocumentAssetError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Yaml(ScenarioYamlError),
}

impl fmt::Display for DialogueDocumentAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "dialogue read failed: {error}"),
            Self::Utf8(error) => write!(formatter, "dialogue is not UTF-8: {error}"),
            Self::Yaml(error) => write!(formatter, "dialogue YAML is invalid: {error}"),
        }
    }
}

impl Error for DialogueDocumentAssetError {}

#[derive(Default, TypePath)]
pub(crate) struct SfxIndexAssetLoader;

impl AssetLoader for SfxIndexAssetLoader {
    type Asset = SfxIndex;
    type Settings = ();
    type Error = SfxIndexAssetError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(SfxIndexAssetError::Io)?;
        let document = std::str::from_utf8(&bytes).map_err(SfxIndexAssetError::Utf8)?;
        scenario_yaml::from_str(document).map_err(SfxIndexAssetError::Yaml)
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

#[derive(Debug)]
pub(crate) enum SfxIndexAssetError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Yaml(ScenarioYamlError),
}

impl fmt::Display for SfxIndexAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "SFX index read failed: {error}"),
            Self::Utf8(error) => write!(formatter, "SFX index is not UTF-8: {error}"),
            Self::Yaml(error) => write!(formatter, "SFX index YAML is invalid: {error}"),
        }
    }
}

impl Error for SfxIndexAssetError {}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;
    use crate::{
        new_game::{NewGameScenario, build_new_game_state},
        runtime_map::RuntimeMapId,
        save_data::NativeSaveEnvelope,
        scenario_dialogue::DialogueDocument,
        scenario_manifest::Manifest,
        scenario_map::MapMetadata,
        scenario_yaml,
        test_support::headless_title_app_with_asset_base,
    };
    use bevy::input::ButtonInput;

    fn complete_linear_dialogue(
        session: &mut DialogueSession,
        flags: &crate::runtime_flags::RuntimeFlags,
    ) -> Vec<DialogueActions> {
        for _ in 0..32 {
            if let DialogueEvent::Apply(actions) = session.confirm(flags) {
                return actions;
            }
        }
        panic!("linear dialogue did not complete");
    }

    #[test]
    fn facing_selection_uses_only_nearest_valid_target() {
        assert!(is_in_facing_direction(
            Position::new(5, 5),
            Position::new(5, 4),
            CardinalDirection::Up
        ));
        assert!(!is_in_facing_direction(
            Position::new(5, 5),
            Position::new(6, 5),
            CardinalDirection::Up
        ));
        assert!(within_range(Position::new(5, 5), Position::new(6, 6), 1.5));
        assert!(!within_range(Position::new(5, 5), Position::new(7, 5), 1.5));
        assert!(
            distance_squared(Position::new(5, 5), Position::new(5, 4))
                < distance_squared(Position::new(5, 5), Position::new(5, 3))
        );
    }

    #[test]
    fn source_pixel_range_reaches_ardel_shopkeepers_across_the_counter() {
        let player_top_left = Vec2::new(192.0, 151.0);
        let shopkeeper_source_position = Vec2::new(192.0, 96.0);

        assert!(is_in_facing_direction_pixels(
            player_top_left,
            shopkeeper_source_position,
            CardinalDirection::Up
        ));
        assert!(within_pixel_range(
            player_top_left,
            shopkeeper_source_position,
            2.5 * 32.0
        ));
    }

    #[test]
    fn elise_join_effect_uses_source_initial_state_and_is_idempotent() {
        let manifest: Manifest = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/manifest.yaml"
        ))
        .unwrap();
        let party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/balance.yaml"
        ))
        .unwrap();
        let mut game = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::ZERO,
        )
        .unwrap();
        let actions: DialogueActions =
            scenario_yaml::from_str("set_flag: npc_elise_joined\njoin_party: elise\n").unwrap();

        apply_dialogue_actions(&actions, &mut game, Some(&party), Some(&balance)).unwrap();
        apply_dialogue_actions(&actions, &mut game, Some(&party), Some(&balance)).unwrap();

        assert!(game.flags().is_set("npc_elise_joined"));
        assert_eq!(game.party().len(), 2);
        let elise = game.party().member("elise").unwrap();
        let source = party
            .party
            .iter()
            .find(|member| member.data().id == "elise")
            .unwrap();
        let expected = RuntimeMember::try_from_catalog(source, &balance.progression).unwrap();
        assert_eq!(elise, &expected);
    }

    #[test]
    fn first_boss_elder_reward_advances_and_round_trips_the_act_two_boundary_once() {
        let manifest: Manifest = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/manifest.yaml"
        ))
        .unwrap();
        let party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/balance.yaml"
        ))
        .unwrap();
        let DialogueDocument::Entries(dialogue) = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/dialogue/elder_intro.yaml"
        ))
        .unwrap() else {
            panic!("elder_intro must remain a field-entry dialogue");
        };
        let mut game = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::ZERO,
        )
        .unwrap();
        game.flags_mut().set("boss_zone01_defeated");
        let initial_hi_potions = game.repository().item_count("hi_potion");
        let initial_tents = game.repository().item_count("tent");

        let mut reward = DialogueSession::resolve(
            "elder_intro",
            Some("Elder Maeve".to_owned()),
            dialogue.clone(),
            game.flags(),
        )
        .unwrap()
        .unwrap();
        assert!(
            reward
                .current_line()
                .starts_with("The forest breathes easier")
        );
        for actions in complete_linear_dialogue(&mut reward, game.flags()) {
            apply_dialogue_actions(&actions, &mut game, Some(&party), Some(&balance)).unwrap();
        }
        assert!(game.flags().is_set("npc_elder_reward_given"));
        assert!(game.flags().is_set("story_act2_started"));
        assert_eq!(
            game.repository().item_count("hi_potion"),
            initial_hi_potions + 2
        );
        assert_eq!(game.repository().item_count("tent"), initial_tents + 1);

        let rewarded_repository = game.repository().clone();
        let mut repeat = DialogueSession::resolve(
            "elder_intro",
            Some("Elder Maeve".to_owned()),
            dialogue,
            game.flags(),
        )
        .unwrap()
        .unwrap();
        assert!(repeat.current_line().starts_with("The plains to the east"));
        for actions in complete_linear_dialogue(&mut repeat, game.flags()) {
            apply_dialogue_actions(&actions, &mut game, Some(&party), Some(&balance)).unwrap();
        }
        assert_eq!(game.repository(), &rewarded_repository);

        let encoded =
            NativeSaveEnvelope::from_game_state(&game, "my_rpg_story", "1.0.0", 1, "Ardel")
                .unwrap()
                .encode()
                .unwrap();
        let (_, restored) =
            NativeSaveEnvelope::decode(&encoded, "my_rpg_story", "1.0.0", &balance).unwrap();
        assert!(restored.flags().is_set("boss_zone01_defeated"));
        assert!(restored.flags().is_set("npc_elder_reward_given"));
        assert!(restored.flags().is_set("story_act2_started"));
        assert_eq!(restored.repository(), &rewarded_repository);
    }

    /// Builds a headless World app wired for interaction, with a fresh game session standing at
    /// `player_position` inside `map_id`, facing down.
    fn interaction_app(map_id: &str, player_position: Position) -> App {
        let mut app = headless_title_app_with_asset_base(
            AppState::World,
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_owned(),
            ScenarioRoot::default(),
        );
        app.add_plugins(WorldInteractionPlugin)
            .insert_resource(WorldTransition::idle_for_test())
            .init_resource::<FieldMenuState>()
            .init_resource::<ServiceUiState>();

        let manifest: Manifest = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/manifest.yaml"
        ))
        .unwrap();
        let party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/balance.yaml"
        ))
        .unwrap();
        let mut game = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::ZERO,
        )
        .unwrap();
        game.map_mut().move_to(
            RuntimeMapId::try_new(map_id).unwrap(),
            player_position,
            CardinalDirection::Down,
        );
        app.insert_resource(game);
        app
    }

    /// Presses Confirm for exactly one simulated input frame (this harness has no
    /// `bevy::input::InputPlugin` to auto-clear `just_pressed` every frame, so it must be cleared
    /// by hand or every later frame would see the same press and endlessly reopen/close the
    /// dialogue), then waits for the resulting interaction request to settle (either a session
    /// opened, or the load failed and nothing did) and gives `play_interaction_sfx` a few more
    /// frames to spawn whatever sound was queued.
    fn press_confirm_and_settle(app: &mut App) {
        for _ in 0..10 {
            app.update();
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(KeyCode::Enter);
        for _ in 0..5_000 {
            if app
                .world()
                .resource::<WorldInteractionState>()
                .request
                .is_none()
            {
                break;
            }
            app.update();
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            app.world()
                .resource::<WorldInteractionState>()
                .request
                .is_none(),
            "interaction request never settled"
        );
        for _ in 0..5 {
            app.update();
        }
    }

    /// Regression for the behavior delta ADR 0007 flagged for W12.3 acceptance: the pinned Python
    /// engine's `DialogueEngine.resolve` returns `None` for a missing dialogue document
    /// (`load_yaml_optional_cached`), and `world_map_scene.py::_try_interact` only opens its
    /// dialogue overlay — the only place a sound is tied to interaction — once `resolve` returns a
    /// match. A missing sign document is therefore a fully silent no-op in the original. Marshland
    /// paints two sign tiles whose dialogue (`sign_zone_03_marshland`) exists in neither
    /// repository. Before this fix, `request_npc_dialogue` queued the "confirm" interaction sound
    /// the instant a sign was selected as a target, before the async asset load even had a chance
    /// to fail — so a missing sign clicked and then showed nothing, unlike the source.
    #[test]
    fn interacting_with_a_missing_sign_dialogue_stays_fully_silent() {
        let mut app = interaction_app("zone_03_marshland", Position::new(5, 5));
        app.world_mut().spawn(WorldSign::for_test(
            "marsh_sign",
            "sign_zone_03_marshland",
            Position::new(5, 6),
        ));

        press_confirm_and_settle(&mut app);

        let state = app.world().resource::<WorldInteractionState>();
        assert!(state.session.is_none(), "a missing sign must not open");
        assert!(state.pending_sounds.is_empty());
        let mut sfx = app.world_mut().query::<&WorldInteractionSfx>();
        assert_eq!(
            sfx.iter(app.world()).count(),
            0,
            "a missing sign dialogue must play no interaction sound at all"
        );
    }

    /// Positive control for the fix above: a sign whose dialogue document does exist still opens
    /// and still plays exactly one `Dialogue` interaction sound, just resolved one (or a few)
    /// frames later than before, once the async load actually confirms a match — never eagerly.
    #[test]
    fn interacting_with_an_authored_sign_dialogue_opens_and_plays_its_sound() {
        let mut app = interaction_app("port_town_harborgate", Position::new(5, 5));
        app.world_mut().spawn(WorldSign::for_test(
            "harborgate_sign",
            "sign_port_town_harborgate",
            Position::new(5, 6),
        ));

        press_confirm_and_settle(&mut app);

        let state = app.world().resource::<WorldInteractionState>();
        assert!(state.failure.is_none(), "{:?}", state.failure);
        assert!(state.session.is_some(), "an authored sign must open");
        let mut sfx = app.world_mut().query::<&WorldInteractionSfx>();
        let sounds = sfx.iter(app.world()).collect::<Vec<_>>();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].logical_event, InteractionSound::Dialogue);
    }

    #[test]
    fn interacting_with_a_ruinwatch_sign_opens_its_authored_dialogue() {
        let mut app = interaction_app("town_03_ruinwatch", Position::new(5, 5));
        app.world_mut().spawn(WorldSign::for_test(
            "ruinwatch_sign",
            "sign_town_03_ruinwatch",
            Position::new(5, 6),
        ));

        press_confirm_and_settle(&mut app);

        let state = app.world().resource::<WorldInteractionState>();
        assert!(state.failure.is_none(), "{:?}", state.failure);
        let session = state.session.as_ref().expect("Ruinwatch sign must speak");
        assert_eq!(session.current_line(), "Notice Board — Ruinwatch");
        let mut sfx = app.world_mut().query::<&WorldInteractionSfx>();
        let sounds = sfx.iter(app.world()).collect::<Vec<_>>();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].logical_event, InteractionSound::Dialogue);
    }

    #[test]
    fn source_forest_box_grants_once_and_reports_open_on_repeat() {
        let manifest: Manifest = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/manifest.yaml"
        ))
        .unwrap();
        let party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/balance.yaml"
        ))
        .unwrap();
        let metadata: MapMetadata = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/maps/zone_01_starting_forest.yaml"
        ))
        .unwrap();
        let mut game = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::ZERO,
        )
        .unwrap();
        let source = &metadata.item_boxes[0];
        let item_box = WorldItemBox::for_test(
            RuntimeMapId::try_new("zone_01_starting_forest").unwrap(),
            source.id.clone(),
            source.position,
            source.loot.clone(),
        );

        let first = open_item_box(&item_box, &mut game);
        let second = open_item_box(&item_box, &mut game);

        assert!(first.opened);
        assert!(first.message.contains("potion ×2"));
        assert!(first.message.contains("antidote ×1"));
        assert_eq!(
            second,
            ItemBoxOutcome {
                message: "This treasure box is already open.".to_owned(),
                opened: false,
            }
        );
        assert_eq!(game.repository().item_count("potion"), 2);
        assert_eq!(game.repository().item_count("antidote"), 1);
        assert_eq!(game.repository().item_count("mc_m"), 3);
        assert_eq!(game.repository().item_count("mc_s"), 10);
        assert!(game.opened_boxes().contains(&item_box.key()));
        assert_eq!(game.opened_boxes().iter().count(), 1);
    }

    #[test]
    fn required_interaction_sounds_resolve_through_source_index() {
        let index: SfxIndex = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/audio/sfx_index.yaml"
        ))
        .unwrap();
        let root = ScenarioRoot::default();

        for sound in [
            InteractionSound::Confirm,
            InteractionSound::Blocked,
            InteractionSound::Dialogue,
            InteractionSound::Box,
            InteractionSound::Cancel,
        ] {
            let path = index.resolve_key(&root, sound.source_key()).unwrap();
            assert!(path.starts_with("scenarios/rusted_kingdoms/assets/audio/sfx/"));
            assert!(path.ends_with(".mp3"));
        }
        assert_eq!(InteractionSound::Dialogue.source_key(), "confirm");
        assert_eq!(InteractionSound::Blocked.source_key(), "denied");
        assert_eq!(InteractionSound::Box.source_key(), "use_item");
    }
}
