//! Full-screen Ember Atlas overlay shared by Sail, Fly, and Teleport.

use bevy::prelude::*;

use crate::{
    action_input::{ActionState, AppAction},
    app_state::AppState,
    field_menu::FieldMenuState,
    field_menu_domain::FieldMenuCatalog,
    game_state::GameState,
    input_bindings::InputBindings,
    key_hints::{HintKey, HintStyle, key_hint},
    menu_chrome::{ScrollWindow, scroll_list_node, spawn_scroll_list},
    options_domain::Device,
    options_store::PlayerOptions,
    runtime_map::RuntimeMapId,
    scenario_class::{AbilityKind, UtilityAbility},
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    scenario_spatial::CardinalDirection,
    scenario_transport::AtlasRegion,
    service_ui::ServiceUiState,
    sfx_cue::{MenuSfx, cue},
    transport_domain::{
        TransportDomain, TransportStatus, TravelAvailability, TravelDeparture, TravelDestination,
        TravelMode, TravelRequest,
    },
    world_encounter::BattleTransition,
    world_interaction::WorldInteractionState,
    world_transition::WorldTransition,
};
const DESTINATION_ROWS: usize = 8;

mod view;
#[cfg(test)]
use view::{
    ATLAS_BACKGROUND, ATLAS_HEIGHT, ATLAS_WIDTH, CLOSED_ACCENT, CURRENT_MARKER, DETAILS_FILL,
    DETAILS_TEXT, EMPTY_NOTICE_BORDER, EMPTY_NOTICE_FILL, EMPTY_NOTICE_TEXT, FLY_ACCENT,
    FOOTER_TEXT, LABEL_FILL, LABEL_TEXT, MAP_BACKGROUND, MAP_FRAME_BORDER, MAP_HEIGHT, MAP_SHADOW,
    MAP_WIDTH, MARKER_FILL, PANEL_BORDER, PANEL_COUNT_TEXT, PANEL_FILL, PANEL_HEADING_TEXT,
    PANEL_SHADOW, REGION_MARKER, ROUTE_CASING, ROW_BORDER, ROW_FILL, ROW_NAME_TEXT,
    ROW_REGION_TEXT, ROW_SELECTED_NAME_TEXT, ROW_UNAVAILABLE_BAR, ROW_UNAVAILABLE_TEXT,
    SAIL_ACCENT, SELECTED_LABEL_FILL, SUBTITLE_TEXT, TAB_BORDER, TAB_FILL, TAB_LOCKED_TEXT,
    TAB_TEXT, TITLE_TEXT, TOKEN_FILL, WARP_ACCENT, WAYPOINT_FILL, destination_count_copy, footer,
};
use view::{TransportRoot, pulse_selected_pin, sync_transport_overlay};

pub(crate) struct TransportUiPlugin;

impl Plugin for TransportUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransportUiState>()
            .add_systems(OnEnter(AppState::World), reset_transport_ui)
            .add_systems(
                Update,
                (
                    handle_transport_input,
                    tick_transport_animation,
                    sync_transport_overlay,
                    pulse_selected_pin,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), cleanup_transport_ui);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TransportPhase {
    #[default]
    Closed,
    Browse,
    Confirm,
    Animating,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WarpContext {
    pub(crate) caster_id: String,
    pub(crate) ability_id: String,
    pub(crate) mp_cost: u32,
}

#[derive(Debug, Resource)]
pub(crate) struct TransportUiState {
    pub(crate) phase: TransportPhase,
    pub(crate) mode: TravelMode,
    pub(crate) selected: usize,
    pub(crate) message: String,
    pub(crate) warp: Option<WarpContext>,
    departure: TravelDeparture,
    /// False for the frame the atlas is opened, so the Confirm press that opened it cannot also
    /// answer it.
    ///
    /// Two openers accept with Confirm and run earlier in `Update` than
    /// [`handle_transport_input`]: the field menu's spell list (`field_menu.rs`; `FieldMenuPlugin`
    /// is registered before `TransportUiPlugin`, and both systems take `ResMut<TransportUiState>`
    /// so the executor cannot interleave them) and the `open_transport` dialogue verb. Without the
    /// latch, that still-`just_pressed` Confirm falls into the browse handler and promotes the atlas
    /// to [`TransportPhase::Confirm`] on the first destination. That phase ignores arrow keys.
    ///
    /// Lives on the state rather than the opener, like [`super::world_interaction`]'s treasure
    /// reveal, so it does not depend on which system runs first.
    confirm_armed: bool,
    pending: Option<TravelRequest>,
    animation_elapsed: f32,
    animation_duration: f32,
    /// Where the destination list was last scrolled.
    scroll: crate::menu_chrome::ScrollMemory,
}

impl Default for TransportUiState {
    fn default() -> Self {
        Self {
            phase: TransportPhase::Closed,
            mode: TravelMode::Closed,
            selected: 0,
            message: String::new(),
            warp: None,
            departure: TravelDeparture::Standard,
            confirm_armed: false,
            pending: None,
            animation_elapsed: 0.0,
            animation_duration: 0.0,
            scroll: crate::menu_chrome::ScrollMemory::default(),
        }
    }
}

impl TransportUiState {
    pub(crate) const fn input_locked(&self) -> bool {
        !matches!(self.phase, TransportPhase::Closed)
    }

    pub(crate) fn open_mode(&mut self, mode: TravelMode) {
        *self = Self {
            phase: TransportPhase::Browse,
            mode,
            ..default()
        };
    }

    pub(crate) fn open_scripted_mode(&mut self, mode: TravelMode) {
        *self = Self {
            phase: TransportPhase::Browse,
            mode,
            departure: TravelDeparture::Scripted,
            ..default()
        };
    }

    pub(crate) fn open_warp(&mut self, caster_id: String, ability_id: String, mp_cost: u32) {
        *self = Self {
            phase: TransportPhase::Browse,
            mode: TravelMode::Warp,
            warp: Some(WarpContext {
                caster_id,
                ability_id,
                mp_cost,
            }),
            ..default()
        };
    }

    fn close(&mut self) {
        *self = Self::default();
    }
}

fn reset_transport_ui(mut state: ResMut<TransportUiState>) {
    state.close();
}

fn cleanup_transport_ui(
    mut commands: Commands,
    roots: Query<Entity, With<TransportRoot>>,
    mut state: ResMut<TransportUiState>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    state.close();
}

#[expect(
    clippy::too_many_arguments,
    reason = "travel confirmation revalidates all mutable runtime dependencies"
)]
pub(crate) fn handle_transport_input(
    actions: Res<ActionState>,
    domain: Res<TransportDomain>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<ResMut<GameState>>,
    transition: Res<WorldTransition>,
    mut state: ResMut<TransportUiState>,
    mut field_menu: Option<ResMut<FieldMenuState>>,
    interaction: Option<Res<WorldInteractionState>>,
    service: Option<Res<ServiceUiState>>,
    battle_transition: Option<Res<BattleTransition>>,
) {
    let Some(game) = game else { return };
    if state.phase == TransportPhase::Closed {
        if actions.just_pressed(AppAction::Travel)
            && domain.status() == TransportStatus::Ready
            && domain.any_convenience_unlocked(game.flags())
            && !transition.input_locked()
            && interaction
                .as_deref()
                .is_none_or(|overlay| !overlay.input_locked())
            && service
                .as_deref()
                .is_none_or(|overlay| !overlay.input_locked())
            && battle_transition
                .as_deref()
                .is_none_or(|overlay| !overlay.input_locked())
        {
            let sail = domain.mode_unlocked(TravelMode::Sail, game.flags());
            let sail_ready = sail
                && domain.availability(TravelMode::Sail, game.map(), game.flags())
                    == TravelAvailability::Available;
            let mode =
                if sail_ready || (sail && !domain.mode_unlocked(TravelMode::Fly, game.flags())) {
                    TravelMode::Sail
                } else {
                    TravelMode::Fly
                };
            state.open_mode(mode);
            if let Some(field_menu) = field_menu.as_deref_mut() {
                field_menu.close();
            }
            state.message = domain
                .availability(mode, game.map(), game.flags())
                .reason()
                .unwrap_or_default()
                .to_owned();
        }
        return;
    }

    // The press that opened the atlas is still `just_pressed` on this frame. Spend it once here
    // rather than letting it answer the screen it just raised; see `confirm_armed`. Back and the
    // arrow keys are unaffected, so cancelling straight out of a freshly opened atlas still works.
    let confirm = actions.just_pressed(AppAction::Confirm) && state.confirm_armed;
    if !state.confirm_armed {
        state.confirm_armed = true;
    }

    if state.phase == TransportPhase::Animating {
        if confirm {
            state.animation_elapsed = state.animation_duration;
        }
        return;
    }
    if actions.just_pressed(AppAction::Back) {
        if state.phase == TransportPhase::Confirm {
            state.phase = TransportPhase::Browse;
            state.message.clear();
        } else {
            state.close();
        }
        return;
    }
    if state.phase == TransportPhase::Confirm {
        if confirm {
            let Some(request) = selected_request(&state, &domain, &catalog, &game) else {
                state.phase = TransportPhase::Browse;
                state.message = "That destination is no longer available.".to_owned();
                return;
            };
            let mode_availability = if request.mode == TravelMode::Warp {
                TravelAvailability::Available
            } else {
                mode_availability(&state, &domain, &game)
            };
            if let Some(reason) = mode_availability.reason() {
                state.phase = TransportPhase::Browse;
                state.message = reason.to_owned();
                return;
            }
            if transition.input_locked() {
                state.phase = TransportPhase::Browse;
                state.message = "A map transition is already active.".to_owned();
                return;
            }
            if request.mode == TravelMode::Warp
                && state.warp.as_ref().is_none_or(|warp| {
                    let valid = game
                        .party()
                        .member(&warp.caster_id)
                        .is_some_and(|member| member.mana() >= warp.mp_cost)
                        && warp_ability(&catalog, &game, warp).is_some();
                    !valid
                })
            {
                state.phase = TransportPhase::Browse;
                state.message = "The selected caster can no longer cast Teleport.".to_owned();
                return;
            }
            state.animation_duration = match request.mode {
                TravelMode::Sail => 1.5,
                TravelMode::Fly => 0.9,
                TravelMode::Warp => 0.7,
                TravelMode::Closed => 0.0,
            };
            state.animation_elapsed = 0.0;
            state.pending = Some(request);
            state.phase = TransportPhase::Animating;
        }
        return;
    }

    if let Some(delta) = actions.menu_navigation_horizontal() {
        let modes = available_modes(&domain, &game, state.warp.is_some());
        if let Some(index) = modes.iter().position(|mode| *mode == state.mode) {
            state.mode = modes[wrapped(index, modes.len(), delta)];
            state.selected = 0;
            state.message = if state.mode == TravelMode::Warp && state.warp.is_none() {
                "Cast Teleport through Spells to choose Warp.".to_owned()
            } else {
                mode_availability(&state, &domain, &game)
                    .reason()
                    .unwrap_or_default()
                    .to_owned()
            };
        }
    }
    let destinations = destinations(&state, &domain, &catalog, &game);
    if let Some(delta) = actions.menu_navigation() {
        state.selected = wrapped(state.selected, destinations.len(), delta);
    }
    if confirm {
        if let Some(destination) = destinations.get(state.selected) {
            if destination.availability == TravelAvailability::Available {
                state.phase = TransportPhase::Confirm;
                state.message = confirm_copy(&state, destination);
            } else {
                state.message = destination
                    .availability
                    .reason()
                    .unwrap_or("Unavailable.")
                    .to_owned();
            }
        } else if state.message.is_empty() {
            state.message = "No visited destinations are available.".to_owned();
        }
    }
}

fn tick_transport_animation(
    time: Res<Time>,
    domain: Res<TransportDomain>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<ResMut<GameState>>,
    mut transition: ResMut<WorldTransition>,
    mut state: ResMut<TransportUiState>,
    mut menu_sfx: MenuSfx,
) {
    if state.phase != TransportPhase::Animating {
        return;
    }
    state.animation_elapsed += time.delta_secs();
    if state.animation_elapsed < state.animation_duration {
        return;
    }
    let Some(mut game) = game else {
        state.close();
        return;
    };
    let Some(request) = state.pending.clone() else {
        state.close();
        return;
    };
    // A second validation at the actual transition boundary makes animation skipping harmless.
    let still_present = destinations(&state, &domain, &catalog, &game)
        .iter()
        .any(|d| {
            d.id == request.destination.id
                && d.map_id == request.destination.map_id
                && d.availability == TravelAvailability::Available
        });
    if !still_present {
        state.phase = TransportPhase::Browse;
        state.pending = None;
        state.message = "That destination is no longer available.".to_owned();
        return;
    }
    let warp = (request.mode == TravelMode::Warp)
        .then(|| state.warp.clone())
        .flatten();
    if request.mode == TravelMode::Warp
        && warp.as_ref().is_none_or(|context| {
            game.party()
                .member(&context.caster_id)
                .is_none_or(|member| member.mana() < context.mp_cost)
                || warp_ability(&catalog, &game, context).is_none()
        })
    {
        state.phase = TransportPhase::Browse;
        state.pending = None;
        state.message = "The selected caster can no longer cast Teleport.".to_owned();
        return;
    }
    let Ok(map_id) = RuntimeMapId::try_new(request.destination.map_id.clone()) else {
        state.phase = TransportPhase::Browse;
        state.message = "The destination map is invalid.".to_owned();
        return;
    };
    if transition.request_destination(
        map_id,
        request.destination.position,
        CardinalDirection::Down,
    ) {
        if let Some(warp) = warp {
            menu_sfx.play(cue::TELEPORT);
            game.party_mut()
                .member_mut(&warp.caster_id)
                .expect("revalidated caster")
                .spend_mana(warp.mp_cost);
        }
        state.close();
    } else {
        menu_sfx.blocked();
        state.phase = TransportPhase::Browse;
        state.pending = None;
        state.message = "A map transition is already active.".to_owned();
    }
}

fn available_modes(
    _domain: &TransportDomain,
    _game: &GameState,
    _warp_context: bool,
) -> Vec<TravelMode> {
    TravelMode::ALL.to_vec()
}

fn destinations(
    state: &TransportUiState,
    domain: &TransportDomain,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> Vec<TravelDestination> {
    match state.mode {
        TravelMode::Sail | TravelMode::Fly => {
            domain.destinations_for(state.mode, game.map(), game.flags(), state.departure)
        }
        TravelMode::Warp if state.warp.is_some() => catalog
            .eligible_warp_destinations(game.map())
            .into_iter()
            .map(|warp| TravelDestination {
                id: warp.map_id.clone(),
                name: warp.name.clone(),
                region_id: domain
                    .region_for_map(&warp.map_id)
                    .map_or_else(|| "unknown".to_owned(), |r| r.id.clone()),
                map_id: warp.map_id.clone(),
                position: warp.position,
                availability: TravelAvailability::Available,
                path: Vec::new(),
            })
            .collect(),
        TravelMode::Warp => Vec::new(),
        TravelMode::Closed => Vec::new(),
    }
}

fn mode_availability(
    state: &TransportUiState,
    domain: &TransportDomain,
    game: &GameState,
) -> TravelAvailability {
    domain.availability_for(state.mode, game.map(), game.flags(), state.departure)
}

fn selected_request(
    state: &TransportUiState,
    domain: &TransportDomain,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> Option<TravelRequest> {
    let destination = destinations(state, domain, catalog, game)
        .get(state.selected)?
        .clone();
    (destination.availability == TravelAvailability::Available).then_some(TravelRequest {
        mode: state.mode,
        destination,
    })
}

fn warp_ability<'a>(
    catalog: &'a FieldMenuCatalog,
    game: &GameState,
    context: &WarpContext,
) -> Option<&'a crate::scenario_class::Ability> {
    let member = game.party().member(&context.caster_id)?;
    catalog
        .class(member.class_id())?
        .abilities
        .iter()
        .find(|ability| {
            ability.id == context.ability_id
                && ability.mp_cost == context.mp_cost
                && ability.unlock_level.get() <= member.level()
                && ability
                    .unlock_flag
                    .as_ref()
                    .is_none_or(|flag| game.flags().is_set(flag))
                && matches!(
                    ability.kind,
                    AbilityKind::Utility(UtilityAbility::Warp { .. })
                )
        })
}

fn confirm_copy(state: &TransportUiState, destination: &TravelDestination) -> String {
    if let Some(warp) = state
        .warp
        .as_ref()
        .filter(|_| state.mode == TravelMode::Warp)
    {
        format!("Warp to {} for {} MP?", destination.name, warp.mp_cost)
    } else {
        format!("{} to {}?", state.mode.label(), destination.name)
    }
}

fn wrapped(index: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        0
    } else {
        (index as isize + delta).rem_euclid(len as isize) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{input_record::NormalizedAction, save_data::tests::fixture_game, sfx_cue::PlaySfx};

    #[test]
    fn atlas_palette_pins_exact_components() {
        assert_eq!(ATLAS_BACKGROUND, Color::srgba(0.025, 0.015, 0.01, 0.985));
        assert_eq!(MAP_FRAME_BORDER, Color::srgb_u8(151, 89, 38));
        assert_eq!(MAP_BACKGROUND, Color::srgb_u8(52, 32, 18));
        assert_eq!(MAP_SHADOW, Color::srgba(0.0, 0.0, 0.0, 0.7));
        assert_eq!(FOOTER_TEXT, Color::srgb_u8(205, 188, 151));
        assert_eq!(CURRENT_MARKER, Color::srgb_u8(255, 220, 126));
        assert_eq!(REGION_MARKER, Color::srgb_u8(224, 171, 84));
        assert_eq!(MARKER_FILL, Color::srgba(0.055, 0.025, 0.012, 0.92));
        assert_eq!(SELECTED_LABEL_FILL, Color::srgba(0.08, 0.035, 0.015, 0.94));
        assert_eq!(LABEL_FILL, Color::srgba(0.07, 0.045, 0.025, 0.82));
        assert_eq!(LABEL_TEXT, Color::srgb_u8(244, 224, 182));
        assert_eq!(PANEL_FILL, Color::srgba(0.085, 0.045, 0.022, 0.97));
        assert_eq!(PANEL_BORDER, Color::srgb_u8(137, 76, 33));
        assert_eq!(PANEL_SHADOW, Color::srgba(0.0, 0.0, 0.0, 0.65));
        assert_eq!(PANEL_HEADING_TEXT, Color::srgb_u8(244, 192, 104));
        assert_eq!(PANEL_COUNT_TEXT, Color::srgb_u8(166, 143, 108));
        assert_eq!(EMPTY_NOTICE_FILL, Color::srgba(0.15, 0.09, 0.04, 0.7));
        assert_eq!(EMPTY_NOTICE_BORDER, Color::srgb_u8(98, 71, 44));
        assert_eq!(EMPTY_NOTICE_TEXT, Color::srgb_u8(196, 177, 143));
        assert_eq!(ROW_FILL, Color::srgba(0.14, 0.08, 0.035, 0.72));
        assert_eq!(ROW_BORDER, Color::srgb_u8(96, 67, 40));
        assert_eq!(ROW_UNAVAILABLE_BAR, Color::srgb_u8(101, 87, 68));
        assert_eq!(ROW_SELECTED_NAME_TEXT, Color::srgb_u8(255, 232, 183));
        assert_eq!(ROW_NAME_TEXT, Color::srgb_u8(221, 207, 176));
        assert_eq!(ROW_UNAVAILABLE_TEXT, Color::srgb_u8(139, 126, 103));
        assert_eq!(ROW_REGION_TEXT, Color::srgb_u8(151, 132, 99));
        assert_eq!(DETAILS_FILL, Color::srgba(0.045, 0.025, 0.014, 0.82));
        assert_eq!(DETAILS_TEXT, Color::srgb_u8(205, 190, 154));
        assert_eq!(TITLE_TEXT, Color::srgb_u8(246, 184, 88));
        assert_eq!(SUBTITLE_TEXT, Color::srgb_u8(184, 151, 103));
        assert_eq!(TAB_FILL, Color::srgba(0.10, 0.06, 0.03, 0.82));
        assert_eq!(TAB_BORDER, Color::srgb_u8(105, 75, 45));
        assert_eq!(TAB_TEXT, Color::srgb_u8(219, 200, 164));
        assert_eq!(TAB_LOCKED_TEXT, Color::srgb_u8(126, 111, 88));
        assert_eq!(TOKEN_FILL, Color::srgba(0.06, 0.025, 0.01, 0.92));
        assert_eq!(WAYPOINT_FILL, Color::srgba(0.05, 0.02, 0.01, 0.94));
        assert_eq!(ROUTE_CASING, Color::srgba(0.055, 0.018, 0.008, 0.88));
        assert_eq!(SAIL_ACCENT, Color::srgb_u8(77, 205, 211));
        assert_eq!(FLY_ACCENT, Color::srgb_u8(247, 188, 72));
        assert_eq!(WARP_ACCENT, Color::srgb_u8(193, 139, 255));
        assert_eq!(CLOSED_ACCENT, Color::srgb_u8(180, 160, 120));
    }

    fn production_domain() -> TransportDomain {
        let catalog =
            crate::scenario_yaml::from_str(include_str!(scenario_file!("data/transport.yaml")))
                .unwrap();
        TransportDomain::try_from_catalog(catalog).unwrap()
    }

    fn arm_warp(
        state: &mut TransportUiState,
        domain: &TransportDomain,
        catalog: &FieldMenuCatalog,
        game: &GameState,
    ) {
        state.open_warp("aric".to_owned(), "teleport".to_owned(), 8);
        let destination = destinations(state, domain, catalog, game)
            .into_iter()
            .next()
            .expect("fixture has one visited destination");
        state.pending = Some(TravelRequest {
            mode: TravelMode::Warp,
            destination,
        });
        state.phase = TransportPhase::Animating;
        state.animation_duration = 0.7;
        state.animation_elapsed = 0.7;
    }

    #[derive(Default, Resource)]
    struct HeardCues(Vec<&'static str>);

    fn collect_cues(mut cues: MessageReader<PlaySfx>, mut heard: ResMut<HeardCues>) {
        heard.0.extend(cues.read().map(|cue| cue.key()));
    }

    #[test]
    fn atlas_layout_is_contained_in_fixed_canvas() {
        assert_eq!((ATLAS_WIDTH, ATLAS_HEIGHT), (1280.0, 720.0));
        const {
            assert!(22.0 + MAP_WIDTH + 18.0 + 370.0 + 22.0 <= ATLAS_WIDTH);
            assert!(MAP_HEIGHT + 2.0 * 22.0 + 100.0 <= ATLAS_HEIGHT);
        }
    }

    #[test]
    fn confirmation_names_mode_destination_and_warp_cost() {
        let destination = TravelDestination {
            id: "ardel".into(),
            name: "Ardel".into(),
            region_id: "heartlands".into(),
            map_id: "town_01_ardel".into(),
            position: crate::scenario_spatial::Position::new(1, 1),
            availability: TravelAvailability::Available,
            path: vec![],
        };
        let mut state = TransportUiState {
            mode: TravelMode::Fly,
            ..default()
        };
        assert_eq!(confirm_copy(&state, &destination), "FLY to Ardel?");
        state.mode = TravelMode::Warp;
        state.warp = Some(WarpContext {
            caster_id: "aric".into(),
            ability_id: "teleport".into(),
            mp_cost: 10,
        });
        assert_eq!(
            confirm_copy(&state, &destination),
            "Warp to Ardel for 10 MP?"
        );
    }

    #[test]
    fn atlas_chrome_uses_only_ascii_text_supported_by_the_scenario_font() {
        let mut state = TransportUiState {
            phase: TransportPhase::Browse,
            mode: TravelMode::Fly,
            message: "Reach open ground before taking flight.".to_owned(),
            ..default()
        };
        for copy in [
            footer(&state, &InputBindings::default()),
            destination_count_copy(0, 12),
            destination_count_copy(0, 1),
            destination_count_copy(0, 0),
        ] {
            assert!(copy.is_ascii(), "unsupported Atlas glyph in {copy:?}");
        }

        state.phase = TransportPhase::Animating;
        assert!(footer(&state, &InputBindings::default()).is_ascii());
    }

    #[test]
    fn cancelling_warp_closes_the_atlas_without_spending_mp() {
        let game = fixture_game();
        let before = crate::save_data::SavePayload::from_game_state(&game).unwrap();
        let initial_mp = game.party().member("aric").unwrap().mana();
        let mut state = TransportUiState::default();
        state.open_warp("aric".to_owned(), "teleport".to_owned(), 8);
        let mut actions = ActionState::default();
        actions.replace_with_normalized(&[NormalizedAction::Back]);

        let mut app = App::new();
        app.insert_resource(actions)
            .insert_resource(production_domain())
            .insert_resource(FieldMenuCatalog::atlas_warp_fixture())
            .insert_resource(game)
            .insert_resource(WorldTransition::idle_for_test())
            .insert_resource(state)
            .add_systems(Update, handle_transport_input);
        app.update();

        assert_eq!(
            app.world().resource::<TransportUiState>().phase,
            TransportPhase::Closed
        );
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .party()
                .member("aric")
                .unwrap()
                .mana(),
            initial_mp
        );
        assert_eq!(
            crate::save_data::SavePayload::from_game_state(app.world().resource::<GameState>())
                .unwrap(),
            before
        );
    }

    #[test]
    fn warp_spends_mp_only_after_the_standard_transition_accepts() {
        let mut game = fixture_game();
        game.flags_mut().set("aric_teleport_unlocked");
        let initial_mp = game.party().member("aric").unwrap().mana();
        let before = crate::save_data::SavePayload::from_game_state(&game).unwrap();
        let domain = production_domain();
        let catalog = FieldMenuCatalog::atlas_warp_fixture();
        let mut state = TransportUiState::default();
        arm_warp(&mut state, &domain, &catalog, &game);

        let mut app = App::new();
        app.add_message::<PlaySfx>()
            .insert_resource(Time::<()>::default())
            .insert_resource(domain)
            .insert_resource(catalog)
            .insert_resource(game)
            .insert_resource(WorldTransition::default())
            .insert_resource(state)
            .init_resource::<HeardCues>()
            .add_systems(Update, (tick_transport_animation, collect_cues).chain());

        app.update();
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .party()
                .member("aric")
                .unwrap()
                .mana(),
            initial_mp,
            "a busy transition must not charge MP"
        );
        assert_eq!(
            crate::save_data::SavePayload::from_game_state(app.world().resource::<GameState>())
                .unwrap(),
            before
        );

        *app.world_mut().resource_mut::<WorldTransition>() = WorldTransition::idle_for_test();
        app.world_mut()
            .resource_mut::<GameState>()
            .flags_mut()
            .unset("aric_teleport_unlocked");
        app.world_mut().resource_mut::<HeardCues>().0.clear();
        app.world_mut()
            .resource_scope(|world, mut state: Mut<TransportUiState>| {
                let domain = world.resource::<TransportDomain>();
                let catalog = world.resource::<FieldMenuCatalog>();
                let game = world.resource::<GameState>();
                arm_warp(&mut state, domain, catalog, game);
            });
        let unavailable =
            crate::save_data::SavePayload::from_game_state(app.world().resource::<GameState>())
                .unwrap();
        app.update();
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .party()
                .member("aric")
                .unwrap()
                .mana(),
            initial_mp,
            "losing the authored unlock during animation must not charge MP"
        );
        assert_eq!(
            crate::save_data::SavePayload::from_game_state(app.world().resource::<GameState>())
                .unwrap(),
            unavailable
        );
        assert!(!app.world().resource::<WorldTransition>().input_locked());

        app.world_mut()
            .resource_mut::<GameState>()
            .flags_mut()
            .set("aric_teleport_unlocked");
        app.world_mut()
            .resource_scope(|world, mut state: Mut<TransportUiState>| {
                let domain = world.resource::<TransportDomain>();
                let catalog = world.resource::<FieldMenuCatalog>();
                let game = world.resource::<GameState>();
                arm_warp(&mut state, domain, catalog, game);
            });
        app.update();

        let transition = app.world().resource::<WorldTransition>();
        assert_eq!(
            transition.pending().unwrap().target_map.as_str(),
            "town_01_ardel"
        );
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .party()
                .member("aric")
                .unwrap()
                .mana(),
            initial_mp - 8
        );
        assert_eq!(
            app.world().resource::<HeardCues>().0,
            [cue::TELEPORT],
            "the unified Atlas must retain Teleport's authored cue"
        );
    }

    fn rebound_confirm_and_back() -> InputBindings {
        let mut bindings = InputBindings::default();
        bindings
            .set_keys(
                crate::input_bindings::BindingTarget::Menu(AppAction::Confirm),
                vec![KeyCode::KeyG],
            )
            .expect("a nonempty row");
        bindings
            .set_keys(
                crate::input_bindings::BindingTarget::Menu(AppAction::Back),
                vec![KeyCode::KeyB],
            )
            .expect("a nonempty row");
        bindings
    }

    #[test]
    fn the_atlas_footer_follows_a_rebound_confirm_and_back() {
        let bindings = rebound_confirm_and_back();
        let mut state = TransportUiState {
            phase: TransportPhase::Browse,
            ..default()
        };
        assert_eq!(
            footer(&state, &bindings),
            "LEFT/RIGHT MODE   UP/DOWN DESTINATION   G TRAVEL   B BACK"
        );
        state.phase = TransportPhase::Confirm;
        state.message = "FLY to Ardel?".to_owned();
        assert_eq!(
            footer(&state, &bindings),
            "G CONFIRM   B BACK     FLY to Ardel?"
        );
        state.phase = TransportPhase::Animating;
        assert_eq!(
            footer(&state, &bindings),
            "G SKIP ANIMATION   Travel is being prepared..."
        );
    }
}
