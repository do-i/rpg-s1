//! One definition of "a World overlay owns the screen".
//!
//! The player already froze whenever a dialogue, menu, service screen, or map
//! transition was up — [`crate::scenario_spatial::cardinal_movement`] checked all
//! five locks inline. Nothing froze the *ambient* World, though, so wandering
//! enemies kept walking while the player stood still reading a message. Opening a
//! treasure box was enough: the reward text appears, the player is locked out, and
//! an enemy strolls into them and starts a battle they had no chance to avoid.
//!
//! Both the player and the World simulation now read the same predicate, so the two
//! cannot drift apart as new overlays are added.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    field_menu::FieldMenuState, service_ui::ServiceUiState, world_encounter::BattleTransition,
    world_interaction::WorldInteractionState, world_transition::WorldTransition,
};

/// Every World overlay that takes input away from the player.
///
/// Each is optional because the World systems run in headless tests that install
/// only the plugins under test.
#[derive(SystemParam)]
pub(crate) struct WorldOverlays<'w> {
    transition: Option<Res<'w, WorldTransition>>,
    battle_transition: Option<Res<'w, BattleTransition>>,
    interaction: Option<Res<'w, WorldInteractionState>>,
    field_menu: Option<Res<'w, FieldMenuState>>,
    service: Option<Res<'w, ServiceUiState>>,
}

impl WorldOverlays<'_> {
    /// True while any overlay holds input, meaning the World must hold still.
    pub(crate) fn any_active(&self) -> bool {
        self.transition
            .as_deref()
            .is_some_and(WorldTransition::input_locked)
            || self
                .battle_transition
                .as_deref()
                .is_some_and(BattleTransition::input_locked)
            || self
                .interaction
                .as_deref()
                .is_some_and(WorldInteractionState::input_locked)
            || self
                .field_menu
                .as_deref()
                .is_some_and(FieldMenuState::input_locked)
            || self
                .service
                .as_deref()
                .is_some_and(ServiceUiState::input_locked)
    }
}

/// Run condition for systems that simulate the World: enemy movement and contact.
///
/// Deliberately does not gate asset loading or the battle hand-off animation, which
/// must keep progressing while an overlay is up.
pub(crate) fn world_simulation_running(overlays: WorldOverlays) -> bool {
    !overlays.any_active()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simulation_running(world: &mut World) -> bool {
        let mut condition = IntoSystem::into_system(world_simulation_running);
        condition.initialize(world);
        condition
            .run((), world)
            .expect("the run condition only reads optional resources")
    }

    #[test]
    fn an_open_dialogue_pauses_the_world_simulation() {
        // The reported bug: reward text from a treasure box locks the player out
        // while an enemy keeps walking, until it collides and forces a battle.
        let mut world = World::new();
        world.insert_resource(WorldInteractionState::default());
        assert!(
            simulation_running(&mut world),
            "an idle World must keep simulating"
        );

        world.insert_resource(WorldInteractionState::dialogue_open_for_tests());
        assert!(
            !simulation_running(&mut world),
            "enemies must hold still while a dialogue owns the screen"
        );
    }

    #[test]
    fn a_world_with_no_overlay_resources_installed_still_simulates() {
        // Headless tests install only the plugins under test, so every overlay
        // resource is optional and absence must never read as "paused".
        let mut world = World::new();
        assert!(simulation_running(&mut world));
    }

    #[test]
    fn one_locked_overlay_pauses_the_world_even_when_the_others_are_idle() {
        let mut world = World::new();
        world.insert_resource(WorldInteractionState::default());
        world.insert_resource(FieldMenuState::default());
        world.insert_resource(ServiceUiState::default());
        world.insert_resource(BattleTransition::default());
        assert!(
            simulation_running(&mut world),
            "every overlay present but unlocked means the World runs"
        );

        world.insert_resource(WorldInteractionState::dialogue_open_for_tests());
        assert!(
            !simulation_running(&mut world),
            "the predicate must be an OR across overlays, not an AND"
        );
    }
}
