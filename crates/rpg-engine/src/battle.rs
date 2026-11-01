//! Deterministic battle domain, resolution, rules, and Bevy presentation.

mod ability;
mod action;
mod badge;
mod enemy_ai;
mod enemy_depth;
mod enemy_pose;
mod fx;
mod item;
mod model;
mod party_ai;
mod resolver;
mod reward_modal;
mod rewards;
mod rules;
mod status;
mod ui;

pub(crate) use ui::BattlePlugin;

#[cfg(test)]
mod tests;
