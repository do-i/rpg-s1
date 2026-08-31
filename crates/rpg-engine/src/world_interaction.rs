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
    dialogue_portrait::{DialoguePortrait, dialogue_portrait_from_sprite},
    engine_settings::EngineSettings,
    field_menu::FieldMenuState,
    field_menu_domain::{FieldMenuCatalog, item_name},
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
    sfx_cue::cue,
    ui_theme::UiTheme,
    world_actor::WorldNpc,
    world_dialogue::{DialogueEvent, DialoguePhase, DialogueSession, apply_flag_actions},
    world_object::{WorldItemBox, WorldSign},
    world_player::{WorldPlayer, WorldPlayerMotion},
    world_transition::WorldTransition,
};

#[cfg(test)]
use crate::dialogue_portrait::dialogue_portrait_crop;

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
                    dismiss_treasure_reveal,
                    play_interaction_sfx,
                    sync_dialogue_overlay,
                    sync_treasure_overlay,
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
    /// The open session's speaker portrait, if the speaker has a world sprite. Signs do not.
    portrait: Option<DialoguePortrait>,
    treasure: Option<TreasureReveal>,
    party: Option<Handle<PartyCatalog>>,
    balance: Option<Handle<BalanceData>>,
    sfx_index: Option<Handle<SfxIndex>>,
    pending_sounds: Vec<InteractionSound>,
    failure: Option<String>,
    sfx_failure: Option<String>,
    suppress_confirm: bool,
    /// Set for the rest of the frame a Back press closes a dialogue, so the same press
    /// cannot fall through to the field menu.
    closed_this_frame: bool,
}

impl WorldInteractionState {
    pub(crate) const fn input_locked(&self) -> bool {
        self.request.is_some()
            || self.session.is_some()
            || self.treasure.is_some()
            || self.closed_this_frame
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
                portrait: None,
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
    /// Resolved when the target is picked, because that is the only point the speaker's world
    /// sprite is in hand; it moves onto the state once the session actually opens.
    portrait: Option<DialoguePortrait>,
    handle: Handle<DialogueDocument>,
}

/// The contents of a treasure box the player just opened, ready to display.
///
/// The source shows this as its own centered modal scene (`engine/world/item_box_scene.py`)
/// rather than as a line of world dialogue, which is why this is a separate state from
/// `session` instead of another `DialogueSession::message`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct TreasureReveal {
    /// One `name ×qty` line per granted stack, in loot order.
    rows: Vec<String>,
    /// False for the frame the box is opened, so the Confirm press that opened the box cannot
    /// also dismiss the reveal it produced. Mirrors `suppress_confirm` for dialogue, but lives
    /// on the reveal so it does not depend on which system runs first.
    armed: bool,
}

#[derive(Component)]
struct WorldDialogueRoot;

#[derive(Component)]
struct TreasureOverlayRoot;

#[derive(Component)]
struct WorldDialogueSpeaker;

#[derive(Component)]
struct WorldDialogueSpeakerPlate;

#[derive(Component)]
struct WorldDialoguePortrait;

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
    catalog: Res<FieldMenuCatalog>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    game: Option<ResMut<GameState>>,
    npcs: Query<(&WorldNpc, &Sprite)>,
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
    let npc = select_npc(player_pixels, facing, npcs.iter()).map(|(target, sprite)| {
        (
            distance_squared(player, target.tile_position()),
            0_u8,
            InteractionTarget::Dialogue {
                id: target.dialogue_id().to_owned(),
                speaker: Some(target.name().to_owned()),
                portrait: dialogue_portrait_from_sprite(sprite, &layouts),
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
                    // A sign has no sprite sheet, so the source passes no portrait either.
                    portrait: None,
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
        let outcome = open_item_box(item_box, &mut game, &catalog);
        state.pending_sounds.push(if outcome.opened {
            InteractionSound::Box
        } else {
            InteractionSound::Blocked
        });
        if outcome.opened {
            state.treasure = Some(TreasureReveal {
                rows: outcome.rows,
                armed: false,
            });
        } else {
            state.session = Some(DialogueSession::message(
                format!("box_{}", item_box.id()),
                Some("Treasure".to_owned()),
                vec![outcome.message],
            ));
            state.suppress_confirm = true;
        }
        state.failure = None;
        return;
    }
    let InteractionTarget::Dialogue {
        id,
        speaker,
        portrait,
    } = target
    else {
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
        portrait,
        handle: asset_server.load(scenario_root.resolve(&logical)),
    });
    // The interaction sound plays once `resolve_dialogue_request` confirms a session actually
    // opens, not here. The pinned Python engine only creates its dialogue overlay after
    // `DialogueEngine.resolve` returns a match (`engine/world/world_map_scene.py::_try_interact`),
    // so a target whose dialogue document is missing (e.g. the two painted `zone_03_marshland`
    // sign tiles — no `sign_zone_03_marshland` document exists in either repository) or whose
    // entries don't match the current
    // flags never makes a sound in the original. Queuing the sound eagerly here, before the async
    // load even resolves, used to make a missing sign click and then show nothing — this player
    // audible difference is why W12.3 acceptance required an explicit parity decision.
    state.failure = None;
}

enum InteractionTarget<'a> {
    Dialogue {
        id: String,
        speaker: Option<String>,
        portrait: Option<DialoguePortrait>,
    },
    Box(&'a WorldItemBox),
}

fn select_npc<'a>(
    player_pixels: Vec2,
    facing: CardinalDirection,
    npcs: impl Iterator<Item = (&'a WorldNpc, &'a Sprite)>,
) -> Option<(&'a WorldNpc, &'a Sprite)> {
    npcs.filter(|(npc, _)| {
        let target = npc.source_pixel_position();
        !npc.map_id().is_empty()
            && is_in_facing_direction_pixels(player_pixels, target, facing)
            && within_pixel_range(player_pixels, target, npc.interaction_range_pixels())
    })
    .min_by(|(left, _), (right, _)| {
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
    /// Shown as world dialogue when the box could not be opened; empty once it was.
    message: String,
    /// The reveal rows, one `name ×qty` per granted stack. Empty when the box did not open.
    rows: Vec<String>,
    opened: bool,
}

/// The source's fallback when an id resolves to no catalog entry: `hi_potion` reads
/// `Hi Potion` rather than leaking the identifier (`item_box_scene.py::_name_for`).
fn humanize_item_id(id: &str) -> String {
    id.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn loot_display_name(catalog: &FieldMenuCatalog, id: &str) -> String {
    catalog
        .item(id)
        .map_or_else(|| humanize_item_id(id), |item| item_name(item).to_owned())
}

fn open_item_box(
    item_box: &WorldItemBox,
    game: &mut GameState,
    catalog: &FieldMenuCatalog,
) -> ItemBoxOutcome {
    let key = item_box.key();
    if game.opened_boxes().contains(&key) {
        return ItemBoxOutcome {
            message: "This treasure box is already open.".to_owned(),
            rows: Vec::new(),
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
        grants.push(format!(
            "{} ×{}",
            loot_display_name(catalog, &item.id),
            item.qty
        ));
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
        grants.push(format!("{} ×{}", loot_display_name(catalog, &id), core.qty));
    }
    game.opened_boxes_mut().record(key);
    if grants.is_empty() {
        // A chest authored with no loot at all. The source still opens its modal and prints one
        // placeholder row rather than a sentence (`item_box_scene.py::_build_lines`).
        grants.push("(empty)".to_owned());
    }
    ItemBoxOutcome {
        message: String::new(),
        rows: grants,
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
    let portrait = request.portrait.clone();
    match DialogueSession::resolve(
        request.id.clone(),
        request.speaker.clone(),
        dialogue.clone(),
        game.flags(),
    ) {
        Ok(Some(session)) => {
            state.session = Some(session);
            state.portrait = portrait;
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

#[expect(
    clippy::too_many_arguments,
    reason = "driving a session reads the clock, input, settings, and every catalog it can apply"
)]
fn drive_dialogue_session(
    time: Res<Time>,
    actions: Res<ActionState>,
    settings: Res<EngineSettings>,
    party_assets: Res<Assets<PartyCatalog>>,
    balance_assets: Res<Assets<BalanceData>>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<ResMut<GameState>>,
    mut service: ResMut<ServiceUiState>,
    mut state: ResMut<WorldInteractionState>,
) {
    let Some(mut game) = game else {
        return;
    };
    // The latch belongs to the frame that set it; releasing it here keeps the field menu
    // from reacting to a Back press this dialogue already consumed.
    state.closed_this_frame = false;
    if state.suppress_confirm {
        state.suppress_confirm = false;
        return;
    }
    let Some(session) = state.session.as_mut() else {
        return;
    };
    session.tick(time.delta_secs(), settings.text_speed);
    if actions.just_pressed(AppAction::Back) {
        session.cancel();
        state.session = None;
        state.portrait = None;
        state.closed_this_frame = true;
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
            if let Err(error) =
                apply_dialogue_actions(&completion, &mut game, party, balance, &catalog)
            {
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
        state.portrait = None;
    }
}

/// Closes the treasure reveal on the next press.
///
/// The source's modal has no cancel path at all -- Enter is its only exit -- but the loot was
/// already granted when the reveal was built, so accepting Back here dismisses the same box
/// without the player being able to lose anything by pressing it.
fn dismiss_treasure_reveal(actions: Res<ActionState>, mut state: ResMut<WorldInteractionState>) {
    let Some(reveal) = state.treasure.as_mut() else {
        return;
    };
    if !reveal.armed {
        reveal.armed = true;
        return;
    }
    if !actions.just_pressed(AppAction::Confirm) && !actions.just_pressed(AppAction::Back) {
        return;
    }
    state.treasure = None;
    state.closed_this_frame = true;
    state.pending_sounds.push(InteractionSound::Confirm);
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
            // Opening a chest reused the item cue as a stand-in; it has its own sample now.
            Self::Box => cue::CHEST_OPEN,
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
    catalog: &FieldMenuCatalog,
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
    let class_id = &source.data().class_id;
    let class = catalog
        .class(class_id)
        .ok_or_else(|| DialogueActionError::UnknownClass(class_id.clone()))?;
    let runtime = RuntimeMember::try_from_catalog(source, class, &balance.progression)
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
    UnknownClass(String),
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
            Self::UnknownClass(id) => {
                write!(formatter, "joining member has unknown class `{id}`")
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
    mut plates: Query<&mut Node, With<WorldDialogueSpeakerPlate>>,
    mut portraits: Query<(&mut ImageNode, &mut Visibility), With<WorldDialoguePortrait>>,
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

/// Box geometry from `engine/dialogue/dialogue_scene.py`.
///
/// The source sizes the box from the screen -- `box_w = screen_w - BOX_MARGIN * 2` -- so it always
/// spans the display. The port had `width: px(920)` hardcoded, which left 340 px of the 1280 px
/// canvas empty to the right of every conversation.
const DIALOGUE_BOX_MARGIN: f32 = 20.0;
const DIALOGUE_BOX_HEIGHT: f32 = 180.0;
const DIALOGUE_BOX_PADDING: f32 = 16.0;
const DIALOGUE_PORTRAIT_SIZE: f32 = 96.0;
/// The source's speaker plate straddles the box's top border rather than sitting inside it.
const DIALOGUE_PLATE_HEIGHT: f32 = 30.0;

fn spawn_dialogue_overlay(commands: &mut Commands, theme: &UiTheme, font: Handle<Font>) {
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
                // pygame draws the box's 2 px border *inside* the rect, so the source's contents
                // start `BOX_PAD` from the box edge. Bevy lays padding out inside the border, so
                // the border's width comes out of the padding to land on the same pixels.
                padding: UiRect::all(px(DIALOGUE_BOX_PADDING - 2.0)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Row,
                column_gap: px(DIALOGUE_BOX_PADDING),
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
            // Name plate: absolutely placed so it can hang over the box's own top border, which
            // is where the source draws it.
            panel
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        // Absolute children sit inside the parent's border, so these insets are
                        // measured from there: 14 px in and half a plate up puts the plate on the
                        // box's top border exactly where the source draws it.
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
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(235, 210, 140)),
                    WorldDialogueSpeaker,
                ));
            // The source draws the portrait's frame whether or not a speaker has a sprite, so a
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
                    column.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(theme.name_entry_input_color),
                        WorldDialogueChoices,
                    ));
                    column.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.into(),
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(theme.name_entry_hint_color),
                        Node {
                            align_self: AlignSelf::FlexEnd,
                            ..default()
                        },
                        WorldDialogueHint,
                    ));
                });
        });
}

/// Panel geometry, copied from `engine/world/item_box_scene.py` so the modal keeps the source's
/// proportions: a fixed-width panel whose height grows one row at a time.
const TREASURE_MODAL_WIDTH: f32 = 520.0;
const TREASURE_ROW_HEIGHT: f32 = 30.0;
const TREASURE_TITLE_HEIGHT: f32 = 42.0;
const TREASURE_HINT_HEIGHT: f32 = 32.0;
const TREASURE_PADDING: f32 = 20.0;

/// Spawns the reveal when a box opens and despawns it when the player dismisses it.
///
/// The rows never change while the modal is up -- the box is looted once, before the reveal
/// exists -- so this spawns the contents rather than re-synchronizing text every frame.
fn sync_treasure_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<WorldInteractionState>,
    roots: Query<Entity, With<TreasureOverlayRoot>>,
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
    spawn_treasure_overlay(&mut commands, font, &reveal.rows);
}

fn spawn_treasure_overlay(commands: &mut Commands, font: Handle<Font>, rows: &[String]) {
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
            // The source dims with a straight sRGB blend at alpha 160/255 (0.627). Bevy composites
            // in linear space, where that same alpha reads visibly lighter -- measured on the
            // forest, a 58-value pixel landed at 34 instead of the source's 22. This alpha
            // reproduces the source's *perceived* darkness instead of its numeric value, the same
            // adjustment the dialogue box already carries (0.94 for the source's 0.863).
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
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
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(240, 220, 140)),
                        Node {
                            height: px(TREASURE_TITLE_HEIGHT),
                            ..default()
                        },
                    ));
                    for row in rows {
                        panel.spawn((
                            Text::new(row.clone()),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(20.0),
                                ..default()
                            },
                            TextColor(Color::srgb_u8(220, 220, 180)),
                            Node {
                                height: px(TREASURE_ROW_HEIGHT),
                                margin: UiRect::left(px(8)),
                                ..default()
                            },
                        ));
                    }
                    panel.spawn((
                        Text::new("ENTER  take"),
                        TextFont {
                            font: font.into(),
                            font_size: FontSize::Px(16.0),
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

/// Every overlay root this module owns, so leaving the World takes all of them down together.
type WorldOverlayRoots = Or<(With<WorldDialogueRoot>, With<TreasureOverlayRoot>)>;

fn cleanup_world_interactions(
    mut commands: Commands,
    roots: Query<Entity, WorldOverlayRoots>,
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
                protagonist_class: &crate::runtime_member::test_class(&manifest.protagonist.class),
            },
            Duration::ZERO,
        )
        .unwrap();
        let actions: DialogueActions =
            scenario_yaml::from_str("set_flag: npc_elise_joined\njoin_party: elise\n").unwrap();

        apply_dialogue_actions(
            &actions,
            &mut game,
            Some(&party),
            Some(&balance),
            &FieldMenuCatalog::production_class_fixture(),
        )
        .unwrap();
        apply_dialogue_actions(
            &actions,
            &mut game,
            Some(&party),
            Some(&balance),
            &FieldMenuCatalog::production_class_fixture(),
        )
        .unwrap();

        assert!(game.flags().is_set("npc_elise_joined"));
        assert_eq!(game.party().len(), 2);
        let elise = game.party().member("elise").unwrap();
        let source = party
            .party
            .iter()
            .find(|member| member.data().id == "elise")
            .unwrap();
        let expected = RuntimeMember::try_from_catalog(
            source,
            FieldMenuCatalog::production_class_fixture()
                .class("cleric")
                .unwrap(),
            &balance.progression,
        )
        .unwrap();
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
                protagonist_class: &crate::runtime_member::test_class(&manifest.protagonist.class),
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
            apply_dialogue_actions(
                &actions,
                &mut game,
                Some(&party),
                Some(&balance),
                &FieldMenuCatalog::production_class_fixture(),
            )
            .unwrap();
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
            apply_dialogue_actions(
                &actions,
                &mut game,
                Some(&party),
                Some(&balance),
                &FieldMenuCatalog::production_class_fixture(),
            )
            .unwrap();
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
            .init_resource::<ServiceUiState>()
            // Opening a treasure box resolves loot ids to display names through the catalog.
            .insert_resource(crate::field_menu_domain::tests::catalog())
            // Speaker portraits read the frame rects out of the NPC sprite's atlas layout.
            // `SpritePlugin` registers this in the real app; the headless harness has no renderer.
            .init_asset::<TextureAtlasLayout>()
            // The typewriter reads its reveal rate from the settings file.
            .init_resource::<EngineSettings>();

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
                protagonist_class: &crate::runtime_member::test_class(&manifest.protagonist.class),
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

    /// Regression for the behavior delta found during W12.3 acceptance: the pinned Python
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
                protagonist_class: &crate::runtime_member::test_class(&manifest.protagonist.class),
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

        let catalog = crate::field_menu_domain::tests::catalog();

        let first = open_item_box(&item_box, &mut game, &catalog);
        let second = open_item_box(&item_box, &mut game, &catalog);

        assert!(first.opened);
        // Display names, not the `potion`/`antidote` ids the player never sees elsewhere.
        assert_eq!(
            first.rows,
            vec![
                "Potion ×2".to_owned(),
                "Antidote ×1".to_owned(),
                "Magic Core (M) ×3".to_owned(),
                "Magic Core (S) ×10".to_owned(),
            ]
        );
        assert_eq!(
            second,
            ItemBoxOutcome {
                message: "This treasure box is already open.".to_owned(),
                rows: Vec::new(),
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

    /// The source crops the head out of the walking sheet rather than loading a portrait asset,
    /// so the numbers here are `get_portrait`'s, reproduced including its `int()` truncation.
    #[test]
    fn a_speaker_portrait_is_the_head_half_of_their_idle_frame() {
        // A 64x64 frame at the third row of the sheet, which is the idle DOWN frame.
        let frame = Rect::new(0.0, 128.0, 64.0, 192.0);

        let head = dialogue_portrait_crop(frame);

        assert_eq!(head.min, Vec2::new(16.0, 138.0), "x = w/4, y = h * 10/64");
        assert_eq!(head.width(), 32.0);
        assert_eq!(head.height(), 32.0);
        assert!(
            head.max.y <= frame.max.y && head.max.x <= frame.max.x,
            "the crop must stay inside its own frame, never bleed into the next one"
        );
    }

    #[test]
    fn a_frame_whose_size_does_not_halve_evenly_truncates_like_the_source() {
        // 33 px: the source's `int(33 * 0.5)` is 16, and `33 // 4` is 8.
        let head = dialogue_portrait_crop(Rect::new(0.0, 0.0, 33.0, 33.0));

        assert_eq!(head.width(), 16.0);
        assert_eq!(head.height(), 16.0);
        assert_eq!(head.min, Vec2::new(8.0, 5.0));
    }

    #[test]
    fn a_sprite_with_no_atlas_yields_no_portrait() {
        let layouts = Assets::<TextureAtlasLayout>::default();

        assert_eq!(
            dialogue_portrait_from_sprite(
                &Sprite::from_color(Color::WHITE, Vec2::splat(64.0)),
                &layouts
            ),
            None,
            "a speaker drawn as a flat placeholder has no head to crop"
        );
    }

    #[test]
    fn an_id_the_catalog_does_not_define_reads_as_words_not_as_an_identifier() {
        assert_eq!(humanize_item_id("hi_potion"), "Hi Potion");
        assert_eq!(humanize_item_id("mimic_key"), "Mimic Key");
        assert_eq!(humanize_item_id("potion"), "Potion");
        // Real ids carry digits and doubled separators; neither may produce an empty word.
        assert_eq!(humanize_item_id("zone_01__drop"), "Zone 01 Drop");
        assert_eq!(
            loot_display_name(&FieldMenuCatalog::default(), "wolf_pelt"),
            "Wolf Pelt"
        );
    }

    /// The press that opens a box must not also dismiss the reveal it produces -- otherwise the
    /// modal appears and vanishes inside one frame and the player never reads their loot.
    #[test]
    fn the_press_that_opens_a_box_cannot_also_dismiss_its_reveal() {
        use crate::action_input::ActionInputPlugin;
        use bevy::input::keyboard::KeyboardInput;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(ActionInputPlugin)
            .init_resource::<WorldInteractionState>()
            .add_message::<KeyboardInput>()
            .add_systems(Update, dismiss_treasure_reveal);

        // Enter is held from the moment the box opens, exactly as it is in play.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.world_mut()
            .resource_mut::<WorldInteractionState>()
            .treasure = Some(TreasureReveal {
            rows: vec!["Potion ×2".to_owned()],
            armed: false,
        });
        app.update();

        let state = app.world().resource::<WorldInteractionState>();
        assert!(state.treasure.is_some(), "the opening press dismissed it");
        assert!(
            state.input_locked(),
            "a reveal must hold World input while it is up"
        );

        // MinimalPlugins carries no input plugin, so releasing and re-pressing is manual.
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear();
            keys.release(KeyCode::Enter);
        }
        app.update();
        assert!(
            app.world()
                .resource::<WorldInteractionState>()
                .treasure
                .is_some(),
            "releasing the key is not a press"
        );

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear();
            keys.press(KeyCode::Enter);
        }
        app.update();

        let state = app.world().resource::<WorldInteractionState>();
        assert!(state.treasure.is_none(), "a fresh press must take the loot");
        assert!(
            state.closed_this_frame,
            "the dismissing press is spent, so it cannot open the field menu"
        );
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
        // Opening a chest used the item cue as a stand-in until the dungeon samples were indexed.
        assert_eq!(InteractionSound::Box.source_key(), cue::CHEST_OPEN);
    }
}
