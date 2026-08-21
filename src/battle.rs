//! Deterministic battle domain, resolution, rules, and Bevy presentation.

mod ability;
mod action;
mod enemy_ai;
mod model;
mod resolver;
mod rules;
mod status;
mod ui;

pub(crate) use ui::BattlePlugin;

#[cfg(test)]
mod tests;
