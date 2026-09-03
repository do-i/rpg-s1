//! Full-screen Ember Atlas overlay shared by Sail, Fly, and Teleport.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::{
    action_input::{ActionState, AppAction},
    app_state::AppState,
    field_menu::FieldMenuState,
    field_menu_domain::FieldMenuCatalog,
    game_state::GameState,
    runtime_map::RuntimeMapId,
    scenario_class::{AbilityKind, UtilityAbility},
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    scenario_spatial::CardinalDirection,
    service_ui::ServiceUiState,
    sfx_cue::{MenuSfx, cue},
    transport_domain::{
        TransportDomain, TransportStatus, TravelAvailability, TravelDestination, TravelMode,
        TravelRequest,
    },
    ui_theme::UiTheme,
    world_encounter::BattleTransition,
    world_interaction::WorldInteractionState,
    world_transition::WorldTransition,
};

const ATLAS_WIDTH: f32 = 1280.0;
const ATLAS_HEIGHT: f32 = 720.0;
const MAP_WIDTH: f32 = 820.0;
const MAP_HEIGHT: f32 = 548.0;
const DESTINATION_ROWS: usize = 8;

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
    /// False for the frame the atlas is opened, so the Confirm press that opened it cannot also
    /// answer it.
    ///
    /// Two of the three openers accept with Confirm and both live in systems that run *earlier*
    /// in `Update` than [`handle_transport_input`]: the field menu's spell list
    /// (`field_menu.rs`, `FieldMenuPlugin` is registered before `TransportUiPlugin`, and the two
    /// systems both take `ResMut<TransportUiState>` so the executor cannot interleave them) and
    /// the `open_transport` dialogue verb. Without the latch that same still-`just_pressed`
    /// Confirm falls straight through into the browse handler, which promotes the atlas to
    /// [`TransportPhase::Confirm`] on whichever destination happens to be first — and that phase
    /// ignores the arrow keys, so the player gets a map they cannot steer.
    ///
    /// Lives on the state rather than on the opener, like [`super::world_interaction`]'s
    /// treasure reveal, so it does not depend on which system runs first.
    confirm_armed: bool,
    pending: Option<TravelRequest>,
    animation_elapsed: f32,
    animation_duration: f32,
}

impl Default for TransportUiState {
    fn default() -> Self {
        Self {
            phase: TransportPhase::Closed,
            mode: TravelMode::Closed,
            selected: 0,
            message: String::new(),
            warp: None,
            confirm_armed: false,
            pending: None,
            animation_elapsed: 0.0,
            animation_duration: 0.0,
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

#[derive(Component)]
struct TransportRoot;

#[derive(Component)]
struct SelectedAtlasPin {
    base_size: f32,
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
                domain.availability(request.mode, game.map(), game.flags())
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
                domain
                    .availability(state.mode, game.map(), game.flags())
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
            domain.destinations(state.mode, game.map(), game.flags())
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

#[derive(SystemParam)]
struct TransportOverlayParams<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    root: Res<'w, ScenarioRoot>,
    inventory: Res<'w, ScenarioInventory>,
    theme: Res<'w, UiTheme>,
    state: Res<'w, TransportUiState>,
    domain: Res<'w, TransportDomain>,
    catalog: Res<'w, FieldMenuCatalog>,
    game: Option<Res<'w, GameState>>,
    roots: Query<'w, 's, Entity, With<TransportRoot>>,
}

fn sync_transport_overlay(mut commands: Commands, params: TransportOverlayParams) {
    let TransportOverlayParams {
        asset_server,
        root,
        inventory,
        theme,
        state,
        domain,
        catalog,
        game,
        roots,
    } = params;
    if state.phase == TransportPhase::Closed {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        return;
    }
    if !state.is_changed() && !domain.is_changed() && game.as_ref().is_none_or(|g| !g.is_changed())
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
    let map_image = asset_server.load(root.resolve(&transport.map_image));
    let rows = destinations(&state, &domain, &catalog, &game);
    let first = window_start(state.selected, rows.len(), DESTINATION_ROWS);
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
            BackgroundColor(Color::srgba(0.025, 0.015, 0.01, 0.985)),
            GlobalZIndex(8_000),
            Pickable::IGNORE,
            TransportRoot,
            Name::new("Ember Atlas"),
        ))
        .with_children(|overlay| {
            overlay.spawn((
                Text::new(format!(
                    "EMBER ATLAS                         {}",
                    mode_header(&state, &domain, &game)
                )),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(28.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(246, 184, 88)),
            ));
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
                            ..default()
                        },
                        ImageNode::new(map_image).with_mode(NodeImageMode::Stretch),
                        BorderColor::all(Color::srgb_u8(151, 89, 38)),
                        BackgroundColor(Color::srgb_u8(52, 32, 18)),
                    ))
                    .with_children(|map| {
                        spawn_route_preview(map, &font, &state, &domain, &game, selected);
                        for region in known_regions {
                            let current = current_region.is_some_and(|r| r.id == region.id);
                            let chosen = selected.is_some_and(|d| d.region_id == region.id);
                            let base_size = if current { 28.0 } else { 22.0 };
                            let mut pin = map.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: percent((region.position.x.get() * 100.0) as f32),
                                    top: percent((region.position.y.get() * 100.0) as f32),
                                    width: px(base_size),
                                    height: px(base_size),
                                    border: UiRect::all(px(if current { 4 } else { 2 })),
                                    border_radius: BorderRadius::all(percent(50)),
                                    ..default()
                                },
                                BorderColor::all(if current {
                                    Color::srgb_u8(255, 214, 92)
                                } else if chosen {
                                    Color::srgb_u8(255, 91, 31)
                                } else {
                                    Color::srgb_u8(238, 175, 74)
                                }),
                                BackgroundColor(Color::srgba(0.12, 0.04, 0.01, 0.72)),
                                Name::new(format!("Atlas pin: {}", region.name)),
                            ));
                            if chosen {
                                pin.insert(SelectedAtlasPin { base_size });
                            }
                            if current {
                                pin.with_children(|outer| {
                                    outer.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: px(5),
                                            top: px(5),
                                            width: px(14),
                                            height: px(14),
                                            border: UiRect::all(px(2)),
                                            border_radius: BorderRadius::all(percent(50)),
                                            ..default()
                                        },
                                        BorderColor::all(Color::srgb_u8(255, 225, 132)),
                                    ));
                                });
                            }
                        }
                    });
                    body.spawn((
                        Node {
                            width: px(370),
                            height: px(MAP_HEIGHT),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(px(14)),
                            row_gap: px(7),
                            border: UiRect::all(px(2)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.10, 0.055, 0.025, 0.96)),
                        BorderColor::all(Color::srgb_u8(137, 76, 33)),
                    ))
                    .with_children(|list| {
                        list.spawn((
                            Text::new("DESTINATIONS"),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(22.0),
                                ..default()
                            },
                            TextColor(Color::srgb_u8(244, 192, 104)),
                        ));
                        for (offset, destination) in
                            rows.iter().skip(first).take(DESTINATION_ROWS).enumerate()
                        {
                            let index = first + offset;
                            let lock = destination
                                .availability
                                .reason()
                                .map_or("", |_| " [LOCKED]");
                            list.spawn((
                                Text::new(format!(
                                    "{} {}{}",
                                    if index == state.selected { ">" } else { " " },
                                    destination.name,
                                    lock
                                )),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(19.0),
                                    ..default()
                                },
                                TextColor(if index == state.selected {
                                    Color::srgb_u8(255, 180, 66)
                                } else {
                                    Color::srgb_u8(228, 215, 180)
                                }),
                            ));
                        }
                        list.spawn((
                            Node {
                                height: px(2),
                                width: percent(100),
                                margin: UiRect::vertical(px(6)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb_u8(122, 71, 34)),
                        ));
                        let detail = selected
                            .map(|d| {
                                format!(
                                    "{}\nVisited · {}\n{}",
                                    domain
                                        .region_for_map(&d.map_id)
                                        .map_or("Unknown region", |r| r.name.as_str()),
                                    state.mode.label(),
                                    d.availability.reason().unwrap_or(match state.mode {
                                        TravelMode::Sail => "No encounters · charted waterway",
                                        TravelMode::Fly => "Direct flight · no encounters",
                                        TravelMode::Warp => "Spell travel · normal MP cost",
                                        _ => "",
                                    })
                                )
                            })
                            .unwrap_or_else(|| state.message.clone());
                        list.spawn((
                            Text::new(detail),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(17.0),
                                ..default()
                            },
                            TextColor(Color::srgb_u8(205, 190, 154)),
                        ));
                    });
                });
            overlay.spawn((
                Text::new(footer(&state)),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(theme.name_entry_hint_color),
            ));
        });
}

fn pulse_selected_pin(time: Res<Time>, mut pins: Query<(&SelectedAtlasPin, &mut Node)>) {
    let pulse = (time.elapsed_secs() * 4.5).sin().mul_add(2.0, 2.0);
    for (pin, mut node) in &mut pins {
        let size = pin.base_size + pulse;
        node.width = px(size);
        node.height = px(size);
    }
}

fn mode_header(state: &TransportUiState, domain: &TransportDomain, game: &GameState) -> String {
    TravelMode::ALL
        .into_iter()
        .map(|mode| {
            let unlocked = match mode {
                TravelMode::Warp => state.warp.is_some(),
                _ => domain.mode_unlocked(mode, game.flags()),
            };
            format!(
                "{}{}{}",
                if mode == state.mode { "[" } else { "" },
                if unlocked {
                    mode.label().to_owned()
                } else {
                    format!("{} LOCKED", mode.label())
                },
                if mode == state.mode { "]" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("  │  ")
}

fn footer(state: &TransportUiState) -> String {
    match state.phase {
        TransportPhase::Browse => format!(
            "←/→ MODE   ↑/↓ DESTINATION   ENTER TRAVEL   ESC BACK{}",
            if state.message.is_empty() {
                String::new()
            } else {
                format!("     {}", state.message)
            }
        ),
        TransportPhase::Confirm => format!("ENTER CONFIRM   ESC BACK     {}", state.message),
        TransportPhase::Animating => {
            "ENTER SKIP ANIMATION   Travel is being prepared...".to_owned()
        }
        TransportPhase::Closed => String::new(),
    }
}

fn spawn_route_preview(
    map: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
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
        vec![
            (
                current.position.x.get() as f32,
                current.position.y.get() as f32,
            ),
            (
                target.position.x.get() as f32,
                target.position.y.get() as f32,
            ),
        ]
    };
    for pair in points.windows(2) {
        spawn_line(map, pair[0], pair[1]);
    }
    if state.phase == TransportPhase::Animating {
        let t = (state.animation_elapsed / state.animation_duration.max(0.001)).clamp(0.0, 1.0);
        if state.mode == TravelMode::Warp && points.len() >= 2 {
            let start = points[0];
            let end = points[points.len() - 1];
            let approach = t * 0.5;
            spawn_animation_token(
                map,
                font,
                "◈",
                start.0 + (end.0 - start.0) * approach,
                start.1 + (end.1 - start.1) * approach,
            );
            spawn_animation_token(
                map,
                font,
                "◈",
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
            spawn_animation_token(
                map,
                font,
                match state.mode {
                    TravelMode::Sail => "◆",
                    TravelMode::Fly => "✦",
                    TravelMode::Warp => "◈",
                    _ => "",
                },
                x,
                y,
            );
        }
    }
}

fn spawn_animation_token(
    map: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    token: &'static str,
    x: f32,
    y: f32,
) {
    map.spawn((
        Text::new(token),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(26.0),
            ..default()
        },
        TextColor(Color::srgb_u8(255, 92, 35)),
        Node {
            position_type: PositionType::Absolute,
            left: percent(x * 100.0),
            top: percent(y * 100.0),
            ..default()
        },
    ));
}

fn spawn_line(map: &mut ChildSpawnerCommands, from: (f32, f32), to: (f32, f32)) {
    let dx = (to.0 - from.0) * MAP_WIDTH;
    let dy = (to.1 - from.1) * MAP_HEIGHT;
    let length = (dx * dx + dy * dy).sqrt();
    let midpoint_x = (from.0 + to.0) * MAP_WIDTH * 0.5;
    let midpoint_y = (from.1 + to.1) * MAP_HEIGHT * 0.5;
    map.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(midpoint_x - length * 0.5),
            top: px(midpoint_y - 2.0),
            width: px(length),
            height: px(4),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 0.28, 0.06, 0.82)),
        UiTransform::from_rotation(Rot2::radians(dy.atan2(dx))),
    ));
}

fn window_start(selected: usize, len: usize, visible: usize) -> usize {
    if len <= visible {
        0
    } else {
        selected
            .saturating_add(1)
            .saturating_sub(visible)
            .min(len - visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{input_record::NormalizedAction, save_data::tests::fixture_game, sfx_cue::PlaySfx};

    fn production_domain() -> TransportDomain {
        let catalog = crate::scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/transport.yaml"
        ))
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
    fn eight_row_window_tracks_selection() {
        assert_eq!(window_start(0, 12, 8), 0);
        assert_eq!(window_start(7, 12, 8), 0);
        assert_eq!(window_start(8, 12, 8), 1);
        assert_eq!(window_start(11, 12, 8), 4);
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
    fn cancelling_warp_closes_the_atlas_without_spending_mp() {
        let game = fixture_game();
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
    }

    #[test]
    fn warp_spends_mp_only_after_the_standard_transition_accepts() {
        let mut game = fixture_game();
        game.flags_mut().set("aric_teleport_unlocked");
        let initial_mp = game.party().member("aric").unwrap().mana();
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
}
