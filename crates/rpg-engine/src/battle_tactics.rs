//! Per-member battle tactics chosen on the field menu's Battle page.
//!
//! A manual member waits for the player's command each turn. An auto member acts on its own
//! with the default command picked here; `Smart` reads the fight (wounds, knockouts, MP, the odds)
//! and varies its choice through the gameplay RNG, so two identical turns need not play the same.
//! Tactics are saved with the member and default to manual, so older saves load unchanged.

use serde::{Deserialize, Serialize};

/// What an auto member does on its turn.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoCommand {
    /// Strike the weakest enemy.
    Attack,
    /// Cast the strongest affordable offensive ability, attacking when none is castable.
    Spell,
    /// Heal, revive, use items, cast, attack, or flee as the battle demands.
    #[default]
    Smart,
}

impl AutoCommand {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Attack => "Attack",
            Self::Spell => "Spell",
            Self::Smart => "Smart",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Attack => "Strikes the weakest enemy every turn.",
            Self::Spell => {
                "Casts the strongest offensive ability it can afford, and attacks when out of MP."
            }
            Self::Smart => {
                "Reads the fight: revives the fallen, heals or uses potions on the wounded, \
                 casts when MP allows, attacks otherwise, and tries to run from a fight it \
                 cannot win. Its choices vary from turn to turn."
            }
        }
    }
}

/// One member's battle tactics.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct BattleTactics {
    /// Whether the member acts without waiting for the player.
    #[serde(default)]
    pub auto: bool,
    /// The default command used while `auto` is on. Kept while manual so toggling back restores it.
    #[serde(default)]
    pub command: AutoCommand,
}

impl BattleTactics {
    /// The command to run this turn, or `None` when the player chooses.
    pub const fn auto_command(self) -> Option<AutoCommand> {
        if self.auto { Some(self.command) } else { None }
    }

    pub const fn mode_label(self) -> &'static str {
        if self.auto { "Auto" } else { "Manual" }
    }

    /// Where these tactics sit in [`TACTIC_CHOICES`].
    pub fn choice_index(self) -> usize {
        TACTIC_CHOICES
            .iter()
            .position(|choice| *choice == self.auto_command())
            .unwrap_or(0)
    }

    /// These tactics with one picker choice applied. Picking Manual keeps the auto command, so
    /// switching back to Auto later restores it.
    pub const fn with_choice(self, choice: Option<AutoCommand>) -> Self {
        match choice {
            None => Self {
                auto: false,
                command: self.command,
            },
            Some(command) => Self {
                auto: true,
                command,
            },
        }
    }
}

/// The Battle page picker's entries in order: Manual, then Auto with each command.
pub const TACTIC_CHOICES: [Option<AutoCommand>; 4] = [
    None,
    Some(AutoCommand::Attack),
    Some(AutoCommand::Spell),
    Some(AutoCommand::Smart),
];

/// A picker entry or a member's current setting, e.g. `Manual` or `Auto - Smart`.
pub fn choice_label(choice: Option<AutoCommand>) -> String {
    choice.map_or_else(
        || "Manual".to_owned(),
        |command| format!("Auto - {}", command.label()),
    )
}

pub const fn choice_description(choice: Option<AutoCommand>) -> &'static str {
    match choice {
        None => "Waits for your command every turn.",
        Some(command) => command.description(),
    }
}

/// How long an automatic turn stays on screen before the battle moves on. Confirm always skips it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TurnPause {
    Instant,
    Half,
    #[default]
    One,
}

impl TurnPause {
    pub const ALL: [Self; 3] = [Self::Instant, Self::Half, Self::One];

    pub const fn duration(self) -> std::time::Duration {
        std::time::Duration::from_millis(match self {
            Self::Instant => 0,
            Self::Half => 500,
            Self::One => 1000,
        })
    }

    /// Both the on-screen label and the options-file value.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Instant => "0s",
            Self::Half => "0.5s",
            Self::One => "1s",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|pause| pause.label() == label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tactics_default_to_manual_and_remember_the_auto_command() {
        let mut tactics = BattleTactics::default();
        assert_eq!(tactics.auto_command(), None);
        tactics.command = AutoCommand::Spell;
        assert_eq!(tactics.auto_command(), None);
        tactics.auto = true;
        assert_eq!(tactics.auto_command(), Some(AutoCommand::Spell));
    }

    #[test]
    fn picking_a_choice_sets_mode_and_command_and_manual_keeps_the_command() {
        let smart = BattleTactics::default().with_choice(Some(AutoCommand::Smart));
        assert_eq!(smart.auto_command(), Some(AutoCommand::Smart));
        assert_eq!(smart.choice_index(), 3);
        let manual = smart.with_choice(None);
        assert_eq!(manual.auto_command(), None);
        assert_eq!(manual.choice_index(), 0);
        assert_eq!(manual.command, AutoCommand::Smart);
        assert_eq!(choice_label(Some(AutoCommand::Spell)), "Auto - Spell");
    }

    #[test]
    fn turn_pause_defaults_to_one_second_and_round_trips_its_label() {
        assert_eq!(TurnPause::default().duration().as_millis(), 1000);
        for pause in TurnPause::ALL {
            assert_eq!(TurnPause::from_label(pause.label()), Some(pause));
        }
        assert_eq!(TurnPause::from_label("2s"), None);
    }

    #[test]
    fn saved_tactics_round_trip_and_missing_fields_default() {
        let tactics = BattleTactics {
            auto: true,
            command: AutoCommand::Attack,
        };
        let yaml = serde_yaml_ng::to_string(&tactics).unwrap();
        assert_eq!(
            serde_yaml_ng::from_str::<BattleTactics>(&yaml).unwrap(),
            tactics
        );
        assert_eq!(
            serde_yaml_ng::from_str::<BattleTactics>("{}").unwrap(),
            BattleTactics::default()
        );
    }
}
