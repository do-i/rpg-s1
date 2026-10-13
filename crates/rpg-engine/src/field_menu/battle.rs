//! Battle page actions: per-member tactics, battle-wide options, and the last battle's log.

use bevy::prelude::*;

use super::{
    input::MenuInput,
    member_at,
    state::{FieldMenuMode, FieldMenuState, wrapped, wrapped_or_zero},
};
use crate::{
    battle_log::LastBattleLog,
    battle_tactics::{TACTIC_CHOICES, TurnPause, choice_description, choice_label},
    engine_settings::EngineSettings,
    game_state::GameState,
    options_store::{OptionsLoadStatus, OptionsStore, PlayerOptions},
    persistence_status::{PersistenceStatus, SaveNotice, persist_player_options},
    sfx_cue::MenuSfx,
};

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    game: &mut ResMut<'_, GameState>,
    input: &MenuInput<'_>,
    menu_settings: &mut MenuSettings<'_>,
    menu_sfx: &mut MenuSfx<'_>,
) {
    match state.mode {
        FieldMenuMode::Browse => {
            if let Some(delta) = input.vertical {
                state.selected = wrapped_or_zero(state.selected, battle_row_count(game), delta);
            }
            if input.confirm {
                let row = battle_row(game, state.selected);
                if row == BattleRow::LastBattle {
                    if menu_settings.battle_log().is_empty() {
                        menu_sfx.blocked();
                    } else {
                        state.log_scroll = 0;
                        state.mode = FieldMenuMode::BattleLog;
                    }
                    return;
                }
                state.battle_choice = battle_row_choices(game, menu_settings.options(), row).1;
                state.mode = FieldMenuMode::BattleTacticPicker;
            }
        }
        FieldMenuMode::BattleLog => {
            if let Some(delta) = input.vertical {
                let last = battle_log_last_scroll(menu_settings.battle_log().lines().len());
                state.log_scroll = state.log_scroll.saturating_add_signed(delta).min(last);
            }
            if let Some(delta) = input.page {
                let last = battle_log_last_scroll(menu_settings.battle_log().lines().len());
                let page = delta * BATTLE_LOG_VISIBLE_LINES as isize;
                state.log_scroll = state.log_scroll.saturating_add_signed(page).min(last);
            }
        }
        FieldMenuMode::BattleTacticPicker => {
            let row = battle_row(game, state.selected);
            if let Some(delta) = input.vertical {
                let count = battle_row_choices(game, menu_settings.options(), row)
                    .0
                    .len();
                state.battle_choice = wrapped(state.battle_choice, count, delta);
            }
            if input.confirm {
                let choice = state.battle_choice;
                menu_settings.apply_battle_choice(game, row, choice, state);
                state.mode = FieldMenuMode::Browse;
            }
        }
        _ => {}
    }
}

/// One line of the Battle page: a party member's orders or one of the battle-wide settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BattleRow {
    Member(usize),
    EnemyTurns,
    TurnPause,
    AfterVictory,
    /// Read-only: opens the previous battle's log.
    LastBattle,
}

/// Members first, then the three settings rows and the last battle's log.
pub(super) fn battle_row_count(game: &GameState) -> usize {
    game.party().len() + 4
}

/// How many log lines the Battle page's log panel shows at once.
pub(super) const BATTLE_LOG_VISIBLE_LINES: usize = 24;

/// The furthest the log can scroll: far enough that its last line sits at the panel's bottom.
pub(super) const fn battle_log_last_scroll(len: usize) -> usize {
    len.saturating_sub(BATTLE_LOG_VISIBLE_LINES)
}

pub(super) fn battle_row(game: &GameState, selected: usize) -> BattleRow {
    match selected.checked_sub(game.party().len()) {
        None => BattleRow::Member(selected),
        Some(0) => BattleRow::EnemyTurns,
        Some(1) => BattleRow::TurnPause,
        Some(2) => BattleRow::AfterVictory,
        Some(_) => BattleRow::LastBattle,
    }
}

/// A row's picker entries as (label, explanation), and the index of the one currently in force.
pub(super) fn battle_row_choices(
    game: &GameState,
    options: &PlayerOptions,
    row: BattleRow,
) -> (Vec<(String, &'static str)>, usize) {
    match row {
        BattleRow::Member(index) => {
            let current =
                member_at(game, index).map_or(0, |member| member.tactics().choice_index());
            let choices = TACTIC_CHOICES
                .into_iter()
                .map(|choice| (choice_label(choice), choice_description(choice)))
                .collect();
            (choices, current)
        }
        BattleRow::EnemyTurns => (
            vec![
                (
                    "Manual".to_owned(),
                    "Each enemy action waits for Enter before the battle moves on.",
                ),
                (
                    "Auto".to_owned(),
                    "Enemy actions move on by themselves after the turn pause. Enter still skips \
                     the wait.",
                ),
            ],
            usize::from(options.auto_enemy_turns),
        ),
        BattleRow::AfterVictory => (
            vec![
                (
                    "Show rewards".to_owned(),
                    "Plays the victory fanfare, then shows EXP, level-ups, and loot before \
                     returning to the world.",
                ),
                (
                    "Return to world".to_owned(),
                    "Skips the fanfare and the reward screen and goes straight back to the map. \
                     EXP, gold, and loot are still awarded.",
                ),
            ],
            usize::from(options.skip_victory_screen),
        ),
        // Nothing to pick: Enter opens the log instead of a picker.
        BattleRow::LastBattle => (Vec::new(), 0),
        BattleRow::TurnPause => (
            TurnPause::ALL
                .into_iter()
                .map(|pause| {
                    (
                        pause.label().to_owned(),
                        "How long an automatic turn stays on screen before the battle moves on. \
                         Applies to auto party members and, when set, to enemy turns. Enter \
                         always skips the wait.",
                    )
                })
                .collect(),
            TurnPause::ALL
                .iter()
                .position(|pause| *pause == options.turn_pause)
                .unwrap_or_default(),
        ),
    }
}

/// The settings the field menu reads, plus the player options it can change and save.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct MenuSettings<'w> {
    pub(super) engine: Res<'w, EngineSettings>,
    options: Option<ResMut<'w, PlayerOptions>>,
    options_store: Option<Res<'w, OptionsStore>>,
    options_load: Option<ResMut<'w, OptionsLoadStatus>>,
    persistence: Option<ResMut<'w, PersistenceStatus>>,
    battle_log: Option<Res<'w, LastBattleLog>>,
}

impl MenuSettings<'_> {
    pub(super) fn options(&self) -> &PlayerOptions {
        static DEFAULT: std::sync::OnceLock<PlayerOptions> = std::sync::OnceLock::new();
        self.options
            .as_deref()
            .unwrap_or_else(|| DEFAULT.get_or_init(PlayerOptions::default))
    }

    pub(super) fn battle_log(&self) -> &LastBattleLog {
        self.battle_log
            .as_deref()
            .unwrap_or_else(|| LastBattleLog::empty())
    }

    /// Applies the picker's highlighted entry to `row`, saving the options file when a
    /// battle-wide setting changed.
    pub(super) fn apply_battle_choice(
        &mut self,
        game: &mut GameState,
        row: BattleRow,
        choice: usize,
        state: &mut FieldMenuState,
    ) {
        if !apply_battle_choice(game, self.options.as_deref_mut(), row, choice) {
            return;
        }
        if let (Some(store), Some(options)) =
            (self.options_store.as_deref(), self.options.as_deref())
        {
            let mut fallback_load = self.options_load.as_deref().cloned().unwrap_or_default();
            let load = self
                .options_load
                .as_deref_mut()
                .unwrap_or(&mut fallback_load);
            let mut fallback_notice = SaveNotice::default();
            let notice = self
                .persistence
                .as_deref_mut()
                .map(|status| &mut status.options)
                .unwrap_or(&mut fallback_notice);
            persist_player_options(store, options, load, notice);
            state.message.clone_from(&notice.message);
        }
    }
}

/// Applies picker entry `choice` to `row`. Returns whether a battle-wide option changed.
pub(super) fn apply_battle_choice(
    game: &mut GameState,
    options: Option<&mut PlayerOptions>,
    row: BattleRow,
    choice: usize,
) -> bool {
    match row {
        BattleRow::Member(index) => {
            let Some(member_id) = member_at(game, index).map(|member| member.id().to_owned())
            else {
                return false;
            };
            let choice = TACTIC_CHOICES[choice.min(TACTIC_CHOICES.len() - 1)];
            let member = game
                .party_mut()
                .member_mut(&member_id)
                .expect("found above");
            member.set_tactics(member.tactics().with_choice(choice));
            false
        }
        BattleRow::LastBattle => false,
        BattleRow::EnemyTurns | BattleRow::TurnPause | BattleRow::AfterVictory => {
            let Some(options) = options else {
                return false;
            };
            match row {
                BattleRow::EnemyTurns => options.auto_enemy_turns = choice == 1,
                BattleRow::AfterVictory => options.skip_victory_screen = choice == 1,
                _ => options.turn_pause = TurnPause::ALL[choice.min(TurnPause::ALL.len() - 1)],
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        battle_tactics::AutoCommand, field_menu::state::FieldMenuScreen,
        save_data::tests::fixture_game,
    };

    #[test]
    fn battle_page_picker_applies_member_tactics_and_battle_wide_options() {
        let mut game = fixture_game();
        let id = game.party().members().next().unwrap().id().to_owned();
        let member = BattleRow::Member(0);
        assert!(!apply_battle_choice(&mut game, None, member, 3));
        let tactics = game.party().member(&id).unwrap().tactics();
        assert_eq!(tactics.auto_command(), Some(AutoCommand::Smart));
        apply_battle_choice(&mut game, None, member, 0);
        assert_eq!(
            game.party().member(&id).unwrap().tactics().auto_command(),
            None
        );

        let party = game.party().len();
        assert_eq!(battle_row(&game, party), BattleRow::EnemyTurns);
        assert_eq!(battle_row(&game, party + 1), BattleRow::TurnPause);
        assert_eq!(battle_row(&game, party + 2), BattleRow::AfterVictory);
        assert_eq!(battle_row(&game, party + 3), BattleRow::LastBattle);
        assert_eq!(battle_row_count(&game), party + 4);
        assert!(!apply_battle_choice(
            &mut game,
            None,
            BattleRow::LastBattle,
            0
        ));
        let mut options = PlayerOptions::default();
        assert!(apply_battle_choice(
            &mut game,
            Some(&mut options),
            BattleRow::EnemyTurns,
            1
        ));
        assert!(options.auto_enemy_turns);
        assert!(apply_battle_choice(
            &mut game,
            Some(&mut options),
            BattleRow::TurnPause,
            0
        ));
        assert_eq!(options.turn_pause, TurnPause::Instant);
        assert!(apply_battle_choice(
            &mut game,
            Some(&mut options),
            BattleRow::AfterVictory,
            1
        ));
        assert!(options.skip_victory_screen);
        assert_eq!(
            battle_row_choices(&game, &options, BattleRow::AfterVictory).1,
            1
        );
        assert_eq!(
            battle_row_choices(&game, &options, BattleRow::TurnPause).1,
            0
        );
        assert_eq!(
            battle_row_choices(&game, &options, BattleRow::EnemyTurns).1,
            1
        );

        let mut state = FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Battle,
            mode: FieldMenuMode::BattleTacticPicker,
            ..default()
        };
        state.back();
        assert_eq!(state.mode, FieldMenuMode::Browse);
        assert_eq!(state.screen, FieldMenuScreen::Battle);
    }
}
