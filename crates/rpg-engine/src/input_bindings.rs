//! The editable binding table, and the conflict policy that governs edits to it.
//!
//! Reading input lives in [`crate::action_input`]; this module owns only what each action is
//! bound to and the rules for changing that. Splitting them keeps the per-frame read path free of
//! editing concerns and lets the binding table be unit-tested with no Bevy app at all.
//!
//! # Categories
//!
//! Bindings are scoped to a *category*, not to one flat table. Menu actions are edge-triggered and
//! movement is level-triggered, so the shipped defaults deliberately place `Up` on both
//! [`AppAction::Up`] and [`MovementAction::Up`] — the same physical key steps a menu in one screen
//! and walks in another, and no screen reads both. A conflict rule that spanned categories would
//! call that shipped, correct arrangement an error, so conflicts are resolved within one category
//! and one device only.

use bevy::input::{gamepad::GamepadButton, keyboard::KeyCode};

/// A semantic menu action shared by application-shell screens.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum AppAction {
    Back,
    Confirm,
    Up,
    Down,
    Left,
    Right,
    Travel,
}

impl AppAction {
    pub(crate) const ALL: [Self; 7] = [
        Self::Back,
        Self::Confirm,
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Travel,
    ];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Back => 0,
            Self::Confirm => 1,
            Self::Up => 2,
            Self::Down => 3,
            Self::Left => 4,
            Self::Right => 5,
            Self::Travel => 6,
        }
    }

    /// The options-file key, which is also the stable identity of the row.
    pub(crate) const fn file_key(self) -> &'static str {
        match self {
            Self::Back => "back",
            Self::Confirm => "confirm",
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Travel => "travel",
        }
    }

    /// The label the options screen shows.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Back => "Back",
            Self::Confirm => "Confirm",
            Self::Up => "Menu Up",
            Self::Down => "Menu Down",
            Self::Left => "Menu Left",
            Self::Right => "Menu Right",
            Self::Travel => "Travel",
        }
    }
}

/// A held world-movement direction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum MovementAction {
    Up,
    Left,
    Down,
    Right,
}

impl MovementAction {
    pub(crate) const ALL: [Self; 4] = [Self::Up, Self::Left, Self::Down, Self::Right];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Up => 0,
            Self::Left => 1,
            Self::Down => 2,
            Self::Right => 3,
        }
    }

    pub(crate) const fn file_key(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Left => "left",
            Self::Down => "down",
            Self::Right => "right",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Up => "Walk Up",
            Self::Left => "Walk Left",
            Self::Down => "Walk Down",
            Self::Right => "Walk Right",
        }
    }
}

/// One editable row: an action, qualified by the category it belongs to.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum BindingTarget {
    Menu(AppAction),
    Movement(MovementAction),
}

impl BindingTarget {
    /// Every row, in the order the options screen lists them.
    pub(crate) fn all() -> Vec<Self> {
        AppAction::ALL
            .into_iter()
            .map(Self::Menu)
            .chain(MovementAction::ALL.into_iter().map(Self::Movement))
            .collect()
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Menu(action) => action.label(),
            Self::Movement(action) => action.label(),
        }
    }

    /// Whether two rows compete for the same physical input.
    const fn same_category(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Menu(_), Self::Menu(_)) | (Self::Movement(_), Self::Movement(_))
        )
    }
}

/// Why an attempted rebind changed nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RebindError {
    /// The row already has this input; rebinding it would be a no-op.
    AlreadyBound,
    /// Taking the input would leave `owner` with no binding at all.
    ///
    /// The approved policy is steal-but-never-unbind: an action that can still be triggered some
    /// other way gives the input up, but the last binding of an action is never taken, because a
    /// player who loses every Confirm key cannot reach the options screen to undo it.
    WouldUnbind { owner: BindingTarget },
    /// The input is outside the port's bindable set, so it could not be written to the file.
    Unbindable,
}

impl RebindError {
    /// The sentence the options screen shows when an edit is refused.
    pub(crate) fn message(self) -> String {
        match self {
            Self::AlreadyBound => "Already bound to this action.".to_owned(),
            Self::WouldUnbind { owner } => {
                format!("{} would be left with no binding.", owner.label())
            }
            Self::Unbindable => "That input cannot be bound.".to_owned(),
        }
    }
}

/// What an accepted rebind did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RebindOutcome {
    /// The row the input was taken from, when the bind resolved a conflict by stealing.
    pub(crate) stolen_from: Option<BindingTarget>,
}

/// Why an attempted unbind changed nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnbindError {
    /// The row is not bound to this input.
    NotBound,
    /// It is the row's only binding, and no action is ever left unbound.
    LastBinding,
}

impl UnbindError {
    pub(crate) fn message(self) -> String {
        match self {
            Self::NotBound => "Not bound to this action.".to_owned(),
            Self::LastBinding => "An action must keep at least one binding.".to_owned(),
        }
    }
}

/// Keyboard and gamepad bindings for every semantic action.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InputBindings {
    menu_keys: [Vec<KeyCode>; 7],
    movement_keys: [Vec<KeyCode>; 4],
    menu_buttons: [Vec<GamepadButton>; 7],
    movement_buttons: [Vec<GamepadButton>; 4],
}

impl Default for InputBindings {
    fn default() -> Self {
        Self {
            menu_keys: [
                vec![KeyCode::Escape],
                vec![KeyCode::Enter, KeyCode::Space, KeyCode::NumpadEnter],
                vec![KeyCode::ArrowUp],
                vec![KeyCode::ArrowDown],
                vec![KeyCode::ArrowLeft],
                vec![KeyCode::ArrowRight],
                vec![KeyCode::KeyT],
            ],
            movement_keys: [
                vec![KeyCode::ArrowUp],
                vec![KeyCode::ArrowLeft],
                vec![KeyCode::ArrowDown],
                vec![KeyCode::ArrowRight],
            ],
            // The face-button defaults follow the platform-neutral convention Bevy's own naming
            // implies: South confirms and East cancels on every pad the backend reports.
            menu_buttons: [
                vec![GamepadButton::East],
                vec![GamepadButton::South],
                vec![GamepadButton::DPadUp],
                vec![GamepadButton::DPadDown],
                vec![GamepadButton::DPadLeft],
                vec![GamepadButton::DPadRight],
                vec![GamepadButton::North],
            ],
            movement_buttons: [
                vec![GamepadButton::DPadUp],
                vec![GamepadButton::DPadLeft],
                vec![GamepadButton::DPadDown],
                vec![GamepadButton::DPadRight],
            ],
        }
    }
}

impl InputBindings {
    pub(crate) fn keys(&self, target: BindingTarget) -> &[KeyCode] {
        match target {
            BindingTarget::Menu(action) => &self.menu_keys[action.index()],
            BindingTarget::Movement(action) => &self.movement_keys[action.index()],
        }
    }

    pub(crate) fn buttons(&self, target: BindingTarget) -> &[GamepadButton] {
        match target {
            BindingTarget::Menu(action) => &self.menu_buttons[action.index()],
            BindingTarget::Movement(action) => &self.movement_buttons[action.index()],
        }
    }

    fn keys_mut(&mut self, target: BindingTarget) -> &mut Vec<KeyCode> {
        match target {
            BindingTarget::Menu(action) => &mut self.menu_keys[action.index()],
            BindingTarget::Movement(action) => &mut self.movement_keys[action.index()],
        }
    }

    fn buttons_mut(&mut self, target: BindingTarget) -> &mut Vec<GamepadButton> {
        match target {
            BindingTarget::Menu(action) => &mut self.menu_buttons[action.index()],
            BindingTarget::Movement(action) => &mut self.movement_buttons[action.index()],
        }
    }

    /// The row that currently owns `key` in `target`'s category, if any.
    fn key_owner(&self, target: BindingTarget, key: KeyCode) -> Option<BindingTarget> {
        BindingTarget::all()
            .into_iter()
            .filter(|candidate| candidate.same_category(target))
            .find(|candidate| self.keys(*candidate).contains(&key))
    }

    fn button_owner(&self, target: BindingTarget, button: GamepadButton) -> Option<BindingTarget> {
        BindingTarget::all()
            .into_iter()
            .filter(|candidate| candidate.same_category(target))
            .find(|candidate| self.buttons(*candidate).contains(&button))
    }

    /// Adds `key` to `target`, applying the approved steal-but-never-unbind policy.
    pub(crate) fn bind_key(
        &mut self,
        target: BindingTarget,
        key: KeyCode,
    ) -> Result<RebindOutcome, RebindError> {
        if crate::input_names::key_name(key).is_none() {
            return Err(RebindError::Unbindable);
        }
        let stolen_from = match self.key_owner(target, key) {
            Some(owner) if owner == target => return Err(RebindError::AlreadyBound),
            Some(owner) if self.keys(owner).len() == 1 => {
                return Err(RebindError::WouldUnbind { owner });
            }
            Some(owner) => {
                self.keys_mut(owner).retain(|bound| *bound != key);
                Some(owner)
            }
            None => None,
        };
        self.keys_mut(target).push(key);
        Ok(RebindOutcome { stolen_from })
    }

    /// Adds `button` to `target`, under the same policy as [`Self::bind_key`].
    pub(crate) fn bind_button(
        &mut self,
        target: BindingTarget,
        button: GamepadButton,
    ) -> Result<RebindOutcome, RebindError> {
        if crate::input_names::gamepad_button_name(button).is_none() {
            return Err(RebindError::Unbindable);
        }
        let stolen_from = match self.button_owner(target, button) {
            Some(owner) if owner == target => return Err(RebindError::AlreadyBound),
            Some(owner) if self.buttons(owner).len() == 1 => {
                return Err(RebindError::WouldUnbind { owner });
            }
            Some(owner) => {
                self.buttons_mut(owner).retain(|bound| *bound != button);
                Some(owner)
            }
            None => None,
        };
        self.buttons_mut(target).push(button);
        Ok(RebindOutcome { stolen_from })
    }

    /// Removes `key` from `target` unless it is the row's last binding.
    pub(crate) fn unbind_key(
        &mut self,
        target: BindingTarget,
        key: KeyCode,
    ) -> Result<(), UnbindError> {
        let bound = self.keys(target);
        if !bound.contains(&key) {
            return Err(UnbindError::NotBound);
        }
        if bound.len() == 1 {
            return Err(UnbindError::LastBinding);
        }
        self.keys_mut(target).retain(|existing| *existing != key);
        Ok(())
    }

    /// Removes `button` from `target` unless it is the row's last binding.
    pub(crate) fn unbind_button(
        &mut self,
        target: BindingTarget,
        button: GamepadButton,
    ) -> Result<(), UnbindError> {
        let bound = self.buttons(target);
        if !bound.contains(&button) {
            return Err(UnbindError::NotBound);
        }
        if bound.len() == 1 {
            return Err(UnbindError::LastBinding);
        }
        self.buttons_mut(target)
            .retain(|existing| *existing != button);
        Ok(())
    }

    /// Replaces one row's keyboard bindings, rejecting an empty or unbindable list.
    ///
    /// This is the loader's entry point, not the screen's: a hand-edited file states a whole row
    /// at once, and a row that survives here is guaranteed to satisfy the same never-unbound
    /// invariant the interactive editor maintains.
    pub(crate) fn set_keys(
        &mut self,
        target: BindingTarget,
        keys: Vec<KeyCode>,
    ) -> Result<(), RebindError> {
        if keys.is_empty() {
            return Err(RebindError::WouldUnbind { owner: target });
        }
        if keys
            .iter()
            .any(|key| crate::input_names::key_name(*key).is_none())
        {
            return Err(RebindError::Unbindable);
        }
        let mut deduplicated = Vec::with_capacity(keys.len());
        for key in keys {
            if !deduplicated.contains(&key) {
                deduplicated.push(key);
            }
        }
        *self.keys_mut(target) = deduplicated;
        Ok(())
    }

    /// Replaces one row's gamepad bindings, rejecting an empty or unbindable list.
    pub(crate) fn set_buttons(
        &mut self,
        target: BindingTarget,
        buttons: Vec<GamepadButton>,
    ) -> Result<(), RebindError> {
        if buttons.is_empty() {
            return Err(RebindError::WouldUnbind { owner: target });
        }
        if buttons
            .iter()
            .any(|button| crate::input_names::gamepad_button_name(*button).is_none())
        {
            return Err(RebindError::Unbindable);
        }
        let mut deduplicated = Vec::with_capacity(buttons.len());
        for button in buttons {
            if !deduplicated.contains(&button) {
                deduplicated.push(button);
            }
        }
        *self.buttons_mut(target) = deduplicated;
        Ok(())
    }

    /// Whether any row differs from the shipped defaults.
    pub(crate) fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIRM: BindingTarget = BindingTarget::Menu(AppAction::Confirm);
    const BACK: BindingTarget = BindingTarget::Menu(AppAction::Back);
    const TRAVEL: BindingTarget = BindingTarget::Menu(AppAction::Travel);
    const WALK_UP: BindingTarget = BindingTarget::Movement(MovementAction::Up);

    #[test]
    fn shipped_defaults_place_one_key_on_two_categories() {
        let bindings = InputBindings::default();

        // The same arrow drives a menu and the world. This is the arrangement the category rule
        // exists to protect, so assert it directly rather than only through the conflict tests.
        assert_eq!(
            bindings.keys(BindingTarget::Menu(AppAction::Up)),
            [KeyCode::ArrowUp]
        );
        assert_eq!(bindings.keys(WALK_UP), [KeyCode::ArrowUp]);
    }

    #[test]
    fn binding_a_free_key_steals_from_nobody() {
        let mut bindings = InputBindings::default();

        let outcome = bindings
            .bind_key(TRAVEL, KeyCode::KeyG)
            .expect("a free key");

        assert_eq!(outcome, RebindOutcome { stolen_from: None });
        assert_eq!(bindings.keys(TRAVEL), [KeyCode::KeyT, KeyCode::KeyG]);
    }

    #[test]
    fn binding_a_contested_key_takes_it_from_an_action_that_keeps_another() {
        let mut bindings = InputBindings::default();

        // Confirm ships with three keys, so it can afford to lose one.
        let outcome = bindings.bind_key(BACK, KeyCode::Enter).expect("a steal");

        assert_eq!(
            outcome,
            RebindOutcome {
                stolen_from: Some(CONFIRM)
            }
        );
        assert_eq!(
            bindings.keys(CONFIRM),
            [KeyCode::Space, KeyCode::NumpadEnter]
        );
        assert_eq!(bindings.keys(BACK), [KeyCode::Escape, KeyCode::Enter]);
    }

    #[test]
    fn binding_an_actions_only_key_is_refused_and_changes_nothing() {
        let mut bindings = InputBindings::default();
        let before = bindings.clone();

        // Back ships with Escape alone, so Escape may not be taken from it.
        let error = bindings
            .bind_key(TRAVEL, KeyCode::Escape)
            .expect_err("the last binding is protected");

        assert_eq!(error, RebindError::WouldUnbind { owner: BACK });
        assert_eq!(bindings, before, "a refused bind must not mutate the table");
        assert_eq!(error.message(), "Back would be left with no binding.");
    }

    #[test]
    fn rebinding_a_key_the_action_already_has_is_refused() {
        let mut bindings = InputBindings::default();
        let before = bindings.clone();

        assert_eq!(
            bindings.bind_key(CONFIRM, KeyCode::Space),
            Err(RebindError::AlreadyBound)
        );
        assert_eq!(bindings, before);
    }

    #[test]
    fn a_key_outside_the_bindable_table_is_refused() {
        let mut bindings = InputBindings::default();

        assert_eq!(
            bindings.bind_key(TRAVEL, KeyCode::MediaPlayPause),
            Err(RebindError::Unbindable)
        );
        assert_eq!(
            bindings.set_keys(TRAVEL, vec![KeyCode::MediaPlayPause]),
            Err(RebindError::Unbindable)
        );
    }

    #[test]
    fn a_menu_conflict_names_the_menu_owner_not_the_movement_one() {
        let mut bindings = InputBindings::default();

        // ArrowUp belongs to menu Up and to walk Up at once. A menu-row edit must see only the
        // menu owner: if the rule were global, walk Up could be reported instead, or the refusal
        // could strip the arrow out of world movement.
        let error = bindings
            .bind_key(TRAVEL, KeyCode::ArrowUp)
            .expect_err("menu Up has ArrowUp as its only key");

        assert_eq!(
            error,
            RebindError::WouldUnbind {
                owner: BindingTarget::Menu(AppAction::Up)
            }
        );
        assert_eq!(bindings.keys(WALK_UP), [KeyCode::ArrowUp]);
    }

    #[test]
    fn a_movement_edit_leaves_the_menu_row_alone() {
        let mut bindings = InputBindings::default();

        // Give walk Up a second key so walk Left can legally steal ArrowUp from it.
        bindings
            .bind_key(WALK_UP, KeyCode::KeyW)
            .expect("W is free in the movement category");
        let outcome = bindings
            .bind_key(
                BindingTarget::Movement(MovementAction::Left),
                KeyCode::ArrowUp,
            )
            .expect("walk Up can spare ArrowUp now");

        assert_eq!(outcome.stolen_from, Some(WALK_UP));
        assert_eq!(bindings.keys(WALK_UP), [KeyCode::KeyW]);
        assert_eq!(
            bindings.keys(BindingTarget::Menu(AppAction::Up)),
            [KeyCode::ArrowUp],
            "the menu row is a different category and must be untouched"
        );
    }

    #[test]
    fn unbinding_respects_the_last_binding_rule() {
        let mut bindings = InputBindings::default();

        assert_eq!(
            bindings.unbind_key(CONFIRM, KeyCode::Space),
            Ok(()),
            "Confirm has three keys"
        );
        assert_eq!(
            bindings.keys(CONFIRM),
            [KeyCode::Enter, KeyCode::NumpadEnter]
        );
        assert_eq!(
            bindings.unbind_key(BACK, KeyCode::Escape),
            Err(UnbindError::LastBinding)
        );
        assert_eq!(
            bindings.unbind_key(BACK, KeyCode::KeyQ),
            Err(UnbindError::NotBound)
        );
    }

    #[test]
    fn gamepad_bindings_follow_the_same_policy_on_their_own_table() {
        let mut bindings = InputBindings::default();

        // Stealing South from Confirm would empty it, exactly as with a keyboard last binding.
        assert_eq!(
            bindings.bind_button(BACK, GamepadButton::South),
            Err(RebindError::WouldUnbind { owner: CONFIRM })
        );
        // A keyboard edit and a gamepad edit are independent tables.
        bindings
            .bind_button(TRAVEL, GamepadButton::Start)
            .expect("Start is free");
        assert_eq!(
            bindings.buttons(TRAVEL),
            [GamepadButton::North, GamepadButton::Start]
        );
        assert_eq!(bindings.keys(TRAVEL), [KeyCode::KeyT], "keys are untouched");
    }

    #[test]
    fn set_keys_rejects_an_empty_row_and_deduplicates() {
        let mut bindings = InputBindings::default();

        assert_eq!(
            bindings.set_keys(TRAVEL, Vec::new()),
            Err(RebindError::WouldUnbind { owner: TRAVEL })
        );
        bindings
            .set_keys(TRAVEL, vec![KeyCode::KeyG, KeyCode::KeyG, KeyCode::KeyH])
            .expect("a nonempty row");
        assert_eq!(bindings.keys(TRAVEL), [KeyCode::KeyG, KeyCode::KeyH]);
    }

    #[test]
    fn is_default_tracks_any_edit() {
        let mut bindings = InputBindings::default();
        assert!(bindings.is_default());

        bindings
            .bind_key(TRAVEL, KeyCode::KeyG)
            .expect("a free key");
        assert!(!bindings.is_default());
    }
}
