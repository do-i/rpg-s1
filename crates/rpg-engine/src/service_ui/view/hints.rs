//! The key hints the service screens print, rendered from the live binding table.
//!
//! Every action key a hint names is one the service input handler reads through
//! [`crate::action_input::ActionState`]. The keys it reads raw (`T`, `Y`, `N`, `TAB`) stay
//! [`HintKey::Fixed`].

use crate::{
    input_bindings::{AppAction, InputBindings},
    key_hints::{HintKey, HintSegment, HintStyle, key_hint},
    options_domain::Device,
};

/// The plain service pages: `LEFT/RIGHT  buy or sell    ENTER / DOWN  item types`.
pub(super) const SERVICE_HINTS: HintStyle = HintStyle {
    key_gap: "  ",
    segment_gap: "    ",
    pair: "/",
    either: " / ",
    label_first: false,
};

/// The rest and exchange confirmations: `Y/ENTER  rest    N/ESC  leave`.
pub(super) const SERVICE_CONFIRM_HINTS: HintStyle = HintStyle {
    key_gap: "  ",
    segment_gap: "    ",
    pair: "/",
    either: "/",
    label_first: false,
};

/// The shop's browse footer and review prompt: `ESC Back    UP / DOWN Browse`.
pub(super) const SHOP_FOOTER_HINTS: HintStyle = HintStyle {
    key_gap: " ",
    segment_gap: "    ",
    pair: " / ",
    either: " / ",
    label_first: false,
};

/// The shop's category-bar reminder: `LEFT / RIGHT MODE   ENTER / DOWN ITEM TYPES`.
pub(super) const SHOP_BAR_HINTS: HintStyle = HintStyle {
    key_gap: " ",
    segment_gap: "   ",
    pair: " / ",
    either: " / ",
    label_first: false,
};

/// The apothecary and forge footers: `ENTER  strike     ESC  back     UP/DOWN  choose another`.
pub(super) const WORKSHOP_HINTS: HintStyle = HintStyle {
    key_gap: "  ",
    segment_gap: "     ",
    pair: "/",
    either: " / ",
    label_first: false,
};

/// The core shop's buttons: `BACK · ESC`.
pub(super) const BUTTON_HINTS: HintStyle = HintStyle {
    key_gap: " · ",
    segment_gap: "",
    pair: "/",
    either: " / ",
    label_first: true,
};

/// The shop dialogs' buttons: `CANCEL  ·  ESC`.
pub(super) const DIALOG_BUTTON_HINTS: HintStyle = HintStyle {
    key_gap: "  ·  ",
    segment_gap: "",
    pair: "/",
    either: " / ",
    label_first: true,
};

pub(super) const UP_DOWN: HintKey = HintKey::Pair(AppAction::Up, AppAction::Down);
pub(super) const LEFT_RIGHT: HintKey = HintKey::Pair(AppAction::Left, AppAction::Right);
pub(super) const ENTER: HintKey = HintKey::Action(AppAction::Confirm);
pub(super) const ESC: HintKey = HintKey::Action(AppAction::Back);
pub(super) const UP: HintKey = HintKey::Action(AppAction::Up);
pub(super) const DOWN: HintKey = HintKey::Action(AppAction::Down);
pub(super) const LEFT: HintKey = HintKey::Action(AppAction::Left);
pub(super) const RIGHT: HintKey = HintKey::Action(AppAction::Right);

/// The default bindings with Confirm moved to `G`, for tests that prove a hint follows a rebind.
#[cfg(test)]
pub(super) fn confirm_on_g() -> InputBindings {
    use crate::input_bindings::BindingTarget;
    use bevy::input::keyboard::KeyCode;

    let mut bindings = InputBindings::default();
    bindings
        .set_keys(BindingTarget::Menu(AppAction::Confirm), vec![KeyCode::KeyG])
        .expect("a nonempty row");
    bindings
}

/// Renders a service hint from the live keyboard bindings.
pub(super) fn service_hint_line(
    bindings: &InputBindings,
    style: &HintStyle,
    segments: &[HintSegment<'_>],
) -> String {
    key_hint(bindings, Device::Keyboard, style, segments)
}
