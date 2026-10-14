//! Hands World contact and scripted battles to Battle through the flash transition.

use bevy::{
    asset::{AssetServer, Assets},
    prelude::*,
};

use crate::{
    app_state::{AppState, AppStateTransitionRequest},
    encounter::{
        EnemyCatalog, PreBattleReturnContext, WorldEnemyReturnState, build_battle_entry,
        build_scripted_battle_entry,
    },
    field_menu_domain::{CatalogStatus, FieldMenuCatalog},
    game_state::GameState,
    scenario_balance::BalanceData,
    scenario_encounter::EncounterZone,
    scenario_map::MapMetadata,
    world_player::{WorldPlayer, WorldPlayerMotion},
};

use super::{
    fixtures::apply_debug_battle_fixture,
    simulation::{enemy_collision_rect, enemy_top_left},
    state::{WorldEncounterState, WorldEncounterStatus, WorldEnemy, fail_encounter},
};

pub(super) const BATTLE_FLASH_SECONDS: f32 = 0.55;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum BattleTransitionPhase {
    #[default]
    Idle,
    Flashing,
    Requested,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct BattleTransition {
    pub(super) phase: BattleTransitionPhase,
    pub(super) elapsed: f32,
}

impl BattleTransition {
    pub(crate) const fn input_locked(&self) -> bool {
        !matches!(self.phase, BattleTransitionPhase::Idle)
    }

    pub(super) fn request(&mut self) -> bool {
        if self.phase != BattleTransitionPhase::Idle {
            return false;
        }
        self.phase = BattleTransitionPhase::Flashing;
        self.elapsed = 0.0;
        true
    }
}

#[derive(Component)]
pub(super) struct BattleFlashOverlay;

pub(super) fn spawn_battle_flash_overlay(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            top: Val::ZERO,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Visibility::Hidden,
        GlobalZIndex(11_000),
        Pickable::IGNORE,
        BattleFlashOverlay,
    ));
}

pub(super) fn cleanup_battle_flash_overlay(
    mut commands: Commands,
    overlays: Query<Entity, With<BattleFlashOverlay>>,
) {
    for entity in &overlays {
        commands.entity(entity).despawn();
    }
}

/// Resolves the loaded scenario's battle balance, falling back to the built-in defaults when no
/// balance asset is present (the same pattern `update_world_enemies` uses for spawner values).
pub(super) fn resolve_battle_balance(
    balances: &Assets<BalanceData>,
) -> rpg_content::scenario_balance::BattleBalance {
    balances.iter().next().map_or_else(
        || BalanceData::default().battle,
        |(_, balance)| balance.battle.clone(),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "contact atomically captures world, data, audio, and transition state"
)]
pub(super) fn detect_enemy_contact(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game: Option<Res<GameState>>,
    enemy_catalog: Option<Res<EnemyCatalog>>,
    item_catalog: Res<FieldMenuCatalog>,
    zones: Res<Assets<EncounterZone>>,
    metadata_assets: Res<Assets<MapMetadata>>,
    balances: Res<Assets<BalanceData>>,
    players: Query<&WorldPlayerMotion, With<WorldPlayer>>,
    mut state: ResMut<WorldEncounterState>,
    mut battle_transition: ResMut<BattleTransition>,
    mut enemies: Query<(Entity, &mut WorldEnemy)>,
) {
    if state.status != WorldEncounterStatus::Spawned || battle_transition.input_locked() {
        return;
    }
    let (Some(game), Some(enemy_catalog)) = (game, enemy_catalog) else {
        return;
    };
    if item_catalog.status() != CatalogStatus::Ready {
        if item_catalog.status() == CatalogStatus::Failed {
            let failure = item_catalog
                .failure()
                .unwrap_or("field item catalog failed to load")
                .to_owned();
            fail_encounter(&mut state, failure);
        }
        return;
    }
    let player = game.map().position();
    let player_rect = players
        .single()
        .map(|motion| motion.collision_rect())
        .unwrap_or_else(|_| enemy_collision_rect(enemy_top_left(player)));
    let Some((contact_entity, contact)) = enemies
        .iter()
        .find(|(_, enemy)| {
            enemy.active && !enemy.engaged && enemy.collision_rect().overlaps(player_rect)
        })
        .map(|(entity, enemy)| (entity, enemy.clone()))
    else {
        return;
    };
    let (Some(zone_handle), Some(metadata_handle)) = (state.zone.as_ref(), state.metadata.as_ref())
    else {
        return;
    };
    let (Some(zone), Some(metadata)) =
        (zones.get(zone_handle), metadata_assets.get(metadata_handle))
    else {
        return;
    };
    let Some(map_id) = game.map().current().map(|map| map.as_str().to_owned()) else {
        return;
    };
    let world_enemies = enemies
        .iter()
        .map(|(_, enemy)| WorldEnemyReturnState {
            encounter_id: enemy.encounter_id.clone(),
            formation: enemy.formation.clone(),
            origin: enemy.origin,
            position: enemy.position,
            facing: enemy.facing,
            boss: enemy.boss,
            chase_range: enemy.chase_range,
            active: enemy.active && enemy.encounter_id != contact.encounter_id,
        })
        .collect();
    let context = PreBattleReturnContext {
        map_id,
        position: player,
        facing: game.map().facing(),
        world_bgm_key: metadata.bgm.clone(),
        world_enemies,
    };
    let entry = match build_battle_entry(
        &contact.encounter_id,
        &contact.formation,
        zone,
        &enemy_catalog,
        &item_catalog,
        game.party(),
        game.repository(),
        game.flags(),
        &resolve_battle_balance(&balances),
        contact.boss,
        context,
    ) {
        Ok(entry) => entry,
        Err(error) => {
            state.failure = Some(error.to_string());
            return;
        }
    };
    let Some(encounter_sfx) = state.encounter_sfx.clone() else {
        state.failure = Some("encounter SFX was not resolved before contact".to_owned());
        return;
    };
    if !battle_transition.request() {
        return;
    }
    commands.spawn((
        AudioPlayer::new(asset_server.load(encounter_sfx)),
        PlaybackSettings::DESPAWN,
    ));
    let Ok((_, mut enemy)) = enemies.get_mut(contact_entity) else {
        return;
    };
    enemy.engaged = true;
    enemy.active = false;
    if let Some(cadence) = state.cadence.as_mut() {
        cadence.reset();
    }
    commands.insert_resource(entry);
}

/// A battle a dialogue asked for with `on_complete.start_battle`.
///
/// Inserted by `world_interaction` only once the dialogue that requested it has closed, so the
/// fight starts after the last line rather than under it.
#[derive(Clone, Debug, Resource)]
pub(crate) struct ScriptedBattleRequest {
    pub(crate) enemy_id: String,
    /// Flags the branch asked for on `start_battle.on_victory`, set only if the player wins.
    pub(crate) victory_flags: Vec<String>,
    /// Present only for a session-only manual-acceptance launch.
    pub(crate) battle_fixture: Option<crate::debug_launch::DebugBattleFixture>,
}

/// Hands a scripted battle to the same entry pipeline a wandering encounter uses.
#[expect(
    clippy::too_many_arguments,
    reason = "assembling a battle entry reads every catalog the encounter path reads"
)]
pub(super) fn start_scripted_battle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    request: Option<Res<ScriptedBattleRequest>>,
    game: Option<Res<GameState>>,
    enemy_catalog: Option<Res<EnemyCatalog>>,
    item_catalog: Res<FieldMenuCatalog>,
    zones: Res<Assets<EncounterZone>>,
    metadata_assets: Res<Assets<MapMetadata>>,
    balances: Res<Assets<BalanceData>>,
    mut state: ResMut<WorldEncounterState>,
    mut battle_transition: ResMut<BattleTransition>,
    enemies: Query<&WorldEnemy>,
) {
    let Some(request) = request else {
        return;
    };
    if battle_transition.input_locked() {
        return;
    }
    match state.status {
        // The map's encounter assets are still settling; the request waits rather than failing.
        WorldEncounterStatus::Idle | WorldEncounterStatus::Loading => return,
        WorldEncounterStatus::Failed => {
            commands.remove_resource::<ScriptedBattleRequest>();
            return;
        }
        WorldEncounterStatus::NoEncounters | WorldEncounterStatus::Spawned => {}
    }
    let enemy_id = request.enemy_id.clone();
    let victory_flags = request.victory_flags.clone();
    let (Some(game), Some(enemy_catalog)) = (game, enemy_catalog) else {
        fail_scripted_battle(
            &mut commands,
            &mut state,
            format!("scripted battle `{enemy_id}` has no enemy catalog on this map"),
        );
        return;
    };
    if item_catalog.status() != CatalogStatus::Ready {
        return;
    }
    let Some(zone) = state.zone.as_ref().and_then(|handle| zones.get(handle)) else {
        fail_scripted_battle(
            &mut commands,
            &mut state,
            format!(
                "scripted battle `{enemy_id}` needs this map's `data/encount/<map_id>.yaml` for \
                 its battle background"
            ),
        );
        return;
    };
    let Some(map_id) = game.map().current().map(|map| map.as_str().to_owned()) else {
        return;
    };
    let world_bgm_key = state
        .metadata
        .as_ref()
        .and_then(|handle| metadata_assets.get(handle))
        .and_then(|metadata| metadata.bgm.clone());
    let context = PreBattleReturnContext {
        map_id,
        position: game.map().position(),
        facing: game.map().facing(),
        world_bgm_key,
        world_enemies: enemies
            .iter()
            .map(|enemy| WorldEnemyReturnState {
                encounter_id: enemy.encounter_id.clone(),
                formation: enemy.formation.clone(),
                origin: enemy.origin,
                position: enemy.position,
                facing: enemy.facing,
                boss: enemy.boss,
                chase_range: enemy.chase_range,
                active: enemy.active,
            })
            .collect(),
    };
    let mut entry = match build_scripted_battle_entry(
        &enemy_id,
        &victory_flags,
        zone,
        &enemy_catalog,
        &item_catalog,
        game.party(),
        game.repository(),
        game.flags(),
        &resolve_battle_balance(&balances),
        context,
    ) {
        Ok(entry) => entry,
        Err(error) => {
            fail_scripted_battle(&mut commands, &mut state, error.to_string());
            return;
        }
    };
    if let Some(fixture) = request.battle_fixture
        && let Err(error) = apply_debug_battle_fixture(&mut entry, fixture, &item_catalog)
    {
        fail_scripted_battle(&mut commands, &mut state, error);
        return;
    }
    if !battle_transition.request() {
        return;
    }
    if let Some(encounter_sfx) = state.encounter_sfx.clone() {
        commands.spawn((
            AudioPlayer::new(asset_server.load(encounter_sfx)),
            PlaybackSettings::DESPAWN,
        ));
    }
    if let Some(cadence) = state.cadence.as_mut() {
        cadence.reset();
    }
    commands.remove_resource::<ScriptedBattleRequest>();
    commands.insert_resource(entry);
}

/// Drops the request so a broken one cannot retry every frame, and records why.
pub(super) fn fail_scripted_battle(
    commands: &mut Commands,
    state: &mut WorldEncounterState,
    failure: String,
) {
    commands.remove_resource::<ScriptedBattleRequest>();
    fail_encounter(state, failure);
}

pub(super) fn advance_battle_transition(
    time: Res<Time>,
    mut transition: ResMut<BattleTransition>,
    mut requests: MessageWriter<AppStateTransitionRequest>,
) {
    if transition.phase != BattleTransitionPhase::Flashing {
        return;
    }
    transition.elapsed += time.delta_secs();
    if transition.elapsed >= BATTLE_FLASH_SECONDS {
        transition.phase = BattleTransitionPhase::Requested;
        requests.write(AppStateTransitionRequest::new(AppState::Battle));
    }
}

pub(super) fn update_battle_flash_overlay(
    transition: Res<BattleTransition>,
    mut overlays: Query<(&mut BackgroundColor, &mut Visibility), With<BattleFlashOverlay>>,
) {
    let alpha = if transition.phase == BattleTransitionPhase::Flashing {
        let progress = (transition.elapsed / BATTLE_FLASH_SECONDS).clamp(0.0, 1.0);
        (progress * std::f32::consts::PI * 3.0).sin().abs()
    } else if transition.phase == BattleTransitionPhase::Requested {
        1.0
    } else {
        0.0
    };
    for (mut color, mut visibility) in &mut overlays {
        color.0 = Color::srgba(1.0, 1.0, 1.0, alpha);
        *visibility = if transition.phase == BattleTransitionPhase::Idle {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_accepts_exactly_one_request_and_locks_input() {
        let state = WorldEncounterState::default();
        assert_eq!(state.status(), WorldEncounterStatus::Idle);
        assert!(state.failure().is_none());
        let mut transition = BattleTransition::default();
        assert!(transition.request());
        assert!(!transition.request());
        assert!(transition.input_locked());
    }
}
