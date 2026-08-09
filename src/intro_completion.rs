//! Applies only the flag portion of completed introductory dialogue actions.

use bevy::prelude::*;

use crate::intro_dialogue::IntroDialogueSet;
use crate::{
    game_state::GameState, intro_dialogue::IntroDialogueCompleted, runtime_flags::RuntimeFlags,
    scenario_dialogue::DialogueActions,
};

pub struct IntroCompletionPlugin;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
pub(crate) enum IntroCompletionSet {
    Flags,
}

impl Plugin for IntroCompletionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_intro_completion
                .in_set(IntroCompletionSet::Flags)
                .after(IntroDialogueSet::Advance),
        );
    }
}
fn apply_intro_completion(
    mut completions: MessageReader<IntroDialogueCompleted>,
    game: Option<ResMut<GameState>>,
) {
    let Some(mut game) = game else {
        completions.clear();
        return;
    };
    for completion in completions.read() {
        apply_flags(game.flags_mut(), completion.on_complete());
    }
}

fn apply_flags(flags: &mut RuntimeFlags, actions: &DialogueActions) {
    if let Some(set_flags) = &actions.set_flag {
        for flag in set_flags.as_slice() {
            flags.set(flag.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario_yaml;

    #[test]
    fn applies_only_authored_one_many_flags_idempotently_and_ignores_other_actions() {
        let actions = scenario_yaml::from_str(
            "set_flag: [one, two]\ngive_items:\n  - id: potion\n    qty: 1\njoin_party: later\n",
        )
        .unwrap();
        let mut flags = crate::runtime_flags::RuntimeFlags::default();
        apply_flags(&mut flags, &actions);
        apply_flags(&mut flags, &actions);
        assert_eq!(flags.iter().collect::<Vec<_>>(), ["one", "two"]);
        let none = crate::scenario_dialogue::DialogueActions::default();
        apply_flags(&mut flags, &none);
        assert_eq!(flags.iter().collect::<Vec<_>>(), ["one", "two"]);
    }
}
