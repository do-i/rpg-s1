//! Deterministic battle domain, resolution, rules, and Bevy presentation.

mod action;
mod model;
mod resolver;
mod rules;
mod ui;

pub(crate) use ui::BattlePlugin;

#[cfg(test)]
mod tests;
