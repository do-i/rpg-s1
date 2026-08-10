//! Facing-aware World interaction, field dialogue loading, effects, and overlay rendering.

use std::{error::Error, fmt};

use bevy::{
    asset::{AssetApp, AssetLoader, LoadContext, LoadState, io::Reader},
    prelude::*,
    reflect::TypePath,
};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::AppState,
    game_state::GameState,
    runtime_member::RuntimeMember,
    scenario_balance::BalanceData,
    scenario_dialogue::{DialogueActions, DialogueDocument},
    scenario_party::PartyCatalog,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::{CardinalDirection, Position},
    scenario_yaml::{self, ScenarioYamlError},
    ui_theme::UiTheme,
    world_actor::WorldNpc,
    world_dialogue::{DialogueEvent, DialoguePhase, DialogueSession, apply_flag_actions},
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
    failure: Option<String>,
}

impl WorldInteractionState {
    pub(crate) const fn input_locked(&self) -> bool {
        self.request.is_some() || self.session.is_some()
    }

    pub(crate) fn session(&self) -> Option<&DialogueSession> {
        self.session.as_ref()
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
    mut state: ResMut<WorldInteractionState>,
) {
    *state = WorldInteractionState {
        party: Some(asset_server.load(scenario_root.resolve(
            &ScenarioRelativePath::try_from("data/party.yaml").expect("canonical party path"),
        ))),
        balance: Some(asset_server.load(scenario_root.resolve(
            &ScenarioRelativePath::try_from("data/balance.yaml").expect("canonical balance path"),
        ))),
        ..default()
    };
}

fn request_npc_dialogue(
    actions: Res<ActionState>,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    transition: Res<WorldTransition>,
    game: Option<Res<GameState>>,
    npcs: Query<&WorldNpc>,
    mut state: ResMut<WorldInteractionState>,
) {
    if state.input_locked()
        || transition.input_locked()
        || !actions.just_pressed(AppAction::Confirm)
    {
        return;
    }
    let Some(game) = game else {
        return;
    };
    let Some(target) = select_npc(game.map().position(), game.map().facing(), npcs.iter()) else {
        return;
    };
    let id = target.dialogue_id().to_owned();
    let logical = format!("data/dialogue/{id}.yaml");
    let Ok(logical) = ScenarioRelativePath::try_from(logical.as_str()) else {
        state.failure = Some(format!("dialogue id `{id}` cannot select a scenario file"));
        return;
    };
    state.request = Some(DialogueRequest {
        id,
        speaker: Some(target.name().to_owned()),
        handle: asset_server.load(scenario_root.resolve(&logical)),
    });
    state.failure = None;
}

fn select_npc<'a>(
    player: Position,
    facing: CardinalDirection,
    npcs: impl Iterator<Item = &'a WorldNpc>,
) -> Option<&'a WorldNpc> {
    npcs.filter(|npc| {
        npc.map_id().len() > 0
            && is_in_facing_direction(player, npc.tile_position(), facing)
            && within_range(player, npc.tile_position(), npc.interaction_range())
    })
    .min_by_key(|npc| distance_squared(player, npc.tile_position()))
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
            state.request = None;
            state.failure = None;
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
    mut state: ResMut<WorldInteractionState>,
) {
    let Some(mut game) = game else {
        return;
    };
    let Some(session) = state.session.as_mut() else {
        return;
    };
    session.tick(time.delta_secs());
    if actions.just_pressed(AppAction::Back) {
        session.cancel();
        state.session = None;
        return;
    }
    if let Some(delta) = actions.menu_navigation() {
        session.move_choice(if delta < 0 { -1 } else { 1 });
    }
    if !actions.just_pressed(AppAction::Confirm) {
        return;
    }
    let event = session.confirm(game.flags());
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

fn apply_dialogue_actions(
    actions: &DialogueActions,
    game: &mut GameState,
    party: Option<&PartyCatalog>,
    balance: Option<&BalanceData>,
) -> Result<(), DialogueActionError> {
    apply_flag_actions(actions, game.flags_mut());
    for gift in &actions.give_items {
        let _outcome = game
            .repository_mut()
            .add_item(&gift.id, gift.qty.get())
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

fn sync_dialogue_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
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
        let font = asset_server.load(
            scenario_root.resolve(
                &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                    .expect("canonical field font path"),
            ),
        );
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
            DialoguePhase::Ready => "▼  ENTER · continue     ESC · close",
            DialoguePhase::Choosing => "▲/▼ · choose     ENTER · select     ESC · close",
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{
        new_game::{NewGameScenario, build_new_game_state},
        scenario_manifest::Manifest,
        scenario_yaml,
    };

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
    fn elise_join_effect_uses_source_initial_state_and_is_idempotent() {
        let manifest: Manifest = scenario_yaml::from_str(include_str!(
            "../assets/scenarios/rusted_kingdoms/manifest.yaml"
        ))
        .unwrap();
        let party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/balance.yaml"
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
}
