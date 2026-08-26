//! Engine-facing spatial module.
//!
//! Scenario-authored value types live in `rpg-content`; runtime movement, collision, and player
//! rendering remain engine systems while the application crate is being separated.

pub use rpg_content::scenario_spatial::*;

pub(crate) mod cardinal_character_atlas;
pub(crate) mod cardinal_movement;
pub(crate) mod collision_occupancy;
pub(crate) mod world_collision;
