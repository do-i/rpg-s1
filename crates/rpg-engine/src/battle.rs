//! Deterministic battle domain, resolution, rules, and Bevy presentation.

mod ability;
mod action;
mod enemy_ai;
mod fx;
mod item;
mod model;
mod resolver;
mod reward_modal;
mod rewards;
mod rules;
mod status;
mod ui;

pub(crate) use ui::BattlePlugin;

#[cfg(test)]
mod tests;
