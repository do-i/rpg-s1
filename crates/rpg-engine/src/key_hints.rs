//! Key-hint lines rendered from the live binding table, so a rebind changes what the hints say.
//!
//! Call sites read the table from [`crate::options_store::PlayerOptions::bindings`] (a
//! `Res<PlayerOptions>` system param) and describe a hint as segments of keys and labels. The
//! upper-case spelling here is for hints only; the options file keeps [`crate::input_names`].

use bevy::input::{gamepad::GamepadButton, keyboard::KeyCode};

use crate::{
    input_bindings::{AppAction, BindingTarget, InputBindings},
    input_names::{gamepad_button_name, key_name},
    options_domain::Device,
};

/// What a key part shows when none of its actions has a binding on the described device.
///
/// The never-unbind rule makes this unreachable today, but a blank would collapse the spacing
/// around it, so the hint says it plainly instead.
pub(crate) const UNBOUND: &str = "UNBOUND";

/// One key named by a hint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HintKey {
    /// The action's first binding on the described device.
    Action(AppAction),
    /// Two opposite actions written as one key, `UP/DOWN`, joined by [`HintStyle::pair`]. An
    /// unbound half is dropped, leaving the other alone; with both unbound the part is
    /// [`UNBOUND`].
    Pair(AppAction, AppAction),
    /// Text shown verbatim: a key the screen reads directly rather than through the binding
    /// table (`M`, `Y`, `TAB`), or a gesture that is not a key (`LEFT EDGE`).
    Fixed(&'static str),
}

/// One hint segment: the keys that do something, as alternatives, and what they do.
pub(crate) type HintSegment<'a> = (&'a [HintKey], &'a str);

/// The separators one family of hint lines uses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HintStyle {
    /// Between a segment's keys and its label.
    pub(crate) key_gap: &'static str,
    /// Between two segments.
    pub(crate) segment_gap: &'static str,
    /// Inside a [`HintKey::Pair`]: `"/"` gives `UP/DOWN`, `" / "` gives `UP / DOWN`.
    pub(crate) pair: &'static str,
    /// Between alternative keys of one segment, `ENTER / Y`.
    pub(crate) either: &'static str,
    /// Whether the label comes before the keys, `BACK · ESC`.
    pub(crate) label_first: bool,
}

/// The field menu's page footers: `UP/DOWN   SELECT      ENTER   CONFIRM      ESC   BACK`.
pub(crate) const FIELD_MENU_HINTS: HintStyle = HintStyle {
    key_gap: "   ",
    segment_gap: "      ",
    pair: "/",
    either: " / ",
    label_first: false,
};

/// Renders a hint line from the live bindings.
///
/// Each segment is its keys joined by [`HintStyle::either`], then [`HintStyle::key_gap`], then its
/// label (the other way round under [`HintStyle::label_first`]); an empty label leaves the keys
/// alone with no gap. Segments are joined by [`HintStyle::segment_gap`]. An action shows only its
/// first binding, so a key added beside the defaults leaves the default text in place.
///
/// `device` picks which column of the table to describe. Callers pass [`Device::Keyboard`] until
/// the game tracks the last-used device; [`Device::Gamepad`] names the first bound button by its
/// positional options-file name in hint case (`SOUTH`, `DPAD UP`).
pub(crate) fn key_hint(
    bindings: &InputBindings,
    device: Device,
    style: &HintStyle,
    segments: &[HintSegment<'_>],
) -> String {
    segments
        .iter()
        .map(|(keys, label)| {
            let keys = hint_keys_text(bindings, device, style, keys);
            match (label.is_empty(), style.label_first) {
                (true, _) => keys,
                (false, false) => format!("{keys}{}{label}", style.key_gap),
                (false, true) => format!("{label}{}{keys}", style.key_gap),
            }
        })
        .collect::<Vec<_>>()
        .join(style.segment_gap)
}

/// The key part of one segment on its own, for a hint written as prose: `ESC cancels.`
///
/// Unbound actions are dropped from the alternatives; if nothing is left the part is [`UNBOUND`].
pub(crate) fn hint_keys_text(
    bindings: &InputBindings,
    device: Device,
    style: &HintStyle,
    keys: &[HintKey],
) -> String {
    let parts = keys
        .iter()
        .filter_map(|key| match *key {
            HintKey::Action(action) => action_text(bindings, device, action),
            HintKey::Pair(first, second) => match (
                action_text(bindings, device, first),
                action_text(bindings, device, second),
            ) {
                (Some(first), Some(second)) => Some(format!("{first}{}{second}", style.pair)),
                (first, second) => first.or(second),
            },
            HintKey::Fixed(text) => Some(text.to_owned()),
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        UNBOUND.to_owned()
    } else {
        parts.join(style.either)
    }
}

fn action_text(bindings: &InputBindings, device: Device, action: AppAction) -> Option<String> {
    let target = BindingTarget::Menu(action);
    match device {
        Device::Keyboard => bindings
            .keys(target)
            .iter()
            .find_map(|key| keyboard_hint_name(*key)),
        Device::Gamepad => bindings
            .buttons(target)
            .iter()
            .find_map(|button| gamepad_hint_name(*button)),
    }
}

/// A key's hint spelling: `ESC` for Escape, otherwise its options-file name split into words and
/// upper-cased (`ENTER`, `UP`, `NUMPAD ENTER`, `LEFT SHIFT`).
fn keyboard_hint_name(key: KeyCode) -> Option<String> {
    if key == KeyCode::Escape {
        return Some("ESC".to_owned());
    }
    key_name(key).map(hint_case)
}

fn gamepad_hint_name(button: GamepadButton) -> Option<String> {
    gamepad_button_name(button).map(hint_case)
}

/// `NumpadEnter` -> `NUMPAD ENTER`: a space where a lower-case letter meets a capital or digit.
fn hint_case(name: &str) -> String {
    let mut text = String::with_capacity(name.len() + 4);
    let mut previous: Option<char> = None;
    for character in name.chars() {
        if previous.is_some_and(|before| before.is_ascii_lowercase())
            && (character.is_ascii_uppercase() || character.is_ascii_digit())
        {
            text.push(' ');
        }
        text.push(character.to_ascii_uppercase());
        previous = Some(character);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_names::{ALL_BINDABLE_GAMEPAD_BUTTONS, ALL_BINDABLE_KEYS};

    use AppAction::{Back, Confirm, Down, Left, Right, Up};
    use HintKey::{Action, Fixed, Pair};

    const UP_DOWN: HintKey = Pair(Up, Down);
    const LEFT_RIGHT: HintKey = Pair(Left, Right);

    fn style(
        key_gap: &'static str,
        segment_gap: &'static str,
        pair: &'static str,
        either: &'static str,
    ) -> HintStyle {
        HintStyle {
            key_gap,
            segment_gap,
            pair,
            either,
            label_first: false,
        }
    }

    fn keyboard(style: &HintStyle, segments: &[HintSegment<'_>]) -> String {
        key_hint(&InputBindings::default(), Device::Keyboard, style, segments)
    }

    #[test]
    fn field_menu_footers_render_byte_identical_to_the_shipped_literals() {
        assert_eq!(
            keyboard(
                &FIELD_MENU_HINTS,
                &[
                    (&[UP_DOWN], "SELECT SLOT"),
                    (&[Action(Confirm)], "SAVE"),
                    (&[Action(Back)], "BACK"),
                    (&[Fixed("M")], "CLOSE"),
                ],
            ),
            "UP/DOWN   SELECT SLOT      ENTER   SAVE      ESC   BACK      M   CLOSE"
        );
        assert_eq!(
            keyboard(
                &FIELD_MENU_HINTS,
                &[
                    (&[Action(Confirm), Fixed("Y")], "OVERWRITE"),
                    (&[Action(Back), Fixed("N")], "CANCEL"),
                ],
            ),
            "ENTER / Y   OVERWRITE      ESC / N   CANCEL"
        );
        assert_eq!(
            keyboard(
                &FIELD_MENU_HINTS,
                &[
                    (&[Fixed("TYPE")], "a-z 0-9 _"),
                    (&[Action(Confirm)], "ADD"),
                    (&[Action(Back)], "BACK"),
                ],
            ),
            "TYPE   a-z 0-9 _      ENTER   ADD      ESC   BACK"
        );
        assert_eq!(
            keyboard(
                &FIELD_MENU_HINTS,
                &[
                    (&[Fixed("ARROWS")], "SELECT SPELL"),
                    (&[Action(Confirm)], "CAST WHEN READY"),
                    (&[Fixed("LEFT EDGE"), Action(Back)], "CASTER"),
                ],
            ),
            "ARROWS   SELECT SPELL      ENTER   CAST WHEN READY      LEFT EDGE / ESC   CASTER"
        );
    }

    #[test]
    fn service_and_compact_footers_render_byte_identical_to_the_shipped_literals() {
        // service_ui/view.rs: two-space key gap, slash pairs, spaced alternatives.
        assert_eq!(
            keyboard(
                &style("  ", "    ", "/", " / "),
                &[
                    (&[LEFT_RIGHT], "buy or sell"),
                    (&[Action(Confirm), Action(Down)], "item types"),
                    (&[Action(Back)], "leave shop"),
                ],
            ),
            "LEFT/RIGHT  buy or sell    ENTER / DOWN  item types    ESC  leave shop"
        );
        assert_eq!(
            keyboard(
                &style("  ", "    ", "/", "/"),
                &[
                    (&[Fixed("Y"), Action(Confirm)], "rest"),
                    (&[Fixed("N"), Action(Back)], "leave"),
                ],
            ),
            "Y/ENTER  rest    N/ESC  leave"
        );
        // service_ui/view.rs: one-space key gap, spaced pairs.
        assert_eq!(
            keyboard(
                &style(" ", "    ", " / ", " / "),
                &[
                    (&[Action(Back)], "Back"),
                    (&[UP_DOWN], "Quantity ±1"),
                    (&[LEFT_RIGHT], "Quantity ±10"),
                    (&[Action(Confirm)], "Review sale"),
                ],
            ),
            "ESC Back    UP / DOWN Quantity ±1    LEFT / RIGHT Quantity ±10    ENTER Review sale"
        );
        // service_ui/view/forge.rs: a five-space segment gap.
        assert_eq!(
            keyboard(
                &style("  ", "     ", "/", " / "),
                &[
                    (&[Action(Confirm)], "strike"),
                    (&[Action(Back)], "back"),
                    (&[UP_DOWN], "choose another"),
                ],
            ),
            "ENTER  strike     ESC  back     UP/DOWN  choose another"
        );
        // field_menu/status.rs: a fixed pair beside a spaced alternative.
        assert_eq!(
            keyboard(
                &style("  ", "    ", "/", " / "),
                &[
                    (&[Fixed("Q/E")], "CHARACTER"),
                    (&[LEFT_RIGHT], "SELECT"),
                    (&[Fixed("TAB"), Action(Confirm)], "POSITION"),
                    (&[Action(Back)], "BACK"),
                ],
            ),
            "Q/E  CHARACTER    LEFT/RIGHT  SELECT    TAB / ENTER  POSITION    ESC  BACK"
        );
        // field_menu/ui/fallback.rs: everything tight.
        assert_eq!(
            keyboard(
                &style(" ", "  ", "/", "/"),
                &[
                    (&[UP_DOWN], "pouch"),
                    (&[Action(Right), Action(Confirm)], "items"),
                    (&[Fixed("H")], "show/hide"),
                    (&[Action(Back)], "back"),
                ],
            ),
            "UP/DOWN pouch  RIGHT/ENTER items  H show/hide  ESC back"
        );
        // transport_ui.rs: one-space key gap, three-space segments.
        assert_eq!(
            keyboard(
                &style(" ", "   ", "/", " / "),
                &[
                    (&[LEFT_RIGHT], "MODE"),
                    (&[UP_DOWN], "DESTINATION"),
                    (&[Action(Confirm)], "TRAVEL"),
                    (&[Action(Back)], "BACK"),
                ],
            ),
            "LEFT/RIGHT MODE   UP/DOWN DESTINATION   ENTER TRAVEL   ESC BACK"
        );
    }

    #[test]
    fn dotted_label_first_bare_and_prose_hints_render_byte_identical() {
        // world_interaction.rs: a dot between key and label.
        assert_eq!(
            keyboard(
                &style(" · ", "     ", "/", " / "),
                &[
                    (&[UP_DOWN], "choose"),
                    (&[Action(Confirm)], "select"),
                    (&[Action(Back)], "close"),
                ],
            ),
            "UP/DOWN · choose     ENTER · select     ESC · close"
        );
        // battle/ui.rs: dots between segments instead.
        assert_eq!(
            keyboard(
                &style(" ", " · ", "/", " / "),
                &[
                    (&[UP_DOWN], "group"),
                    (&[Action(Right)], "spells"),
                    (&[Action(Back)], "commands"),
                ],
            ),
            "UP/DOWN group · RIGHT spells · ESC commands"
        );
        // service_ui/view/core_shop.rs and view.rs: the label leads.
        let label_first = |key_gap| HintStyle {
            label_first: true,
            ..style(key_gap, "", "/", " / ")
        };
        assert_eq!(
            keyboard(&label_first(" · "), &[(&[Action(Back)], "BACK")]),
            "BACK · ESC"
        );
        assert_eq!(
            keyboard(
                &label_first("  ·  "),
                &[(&[Action(Confirm), Action(Back)], "OK")]
            ),
            "OK  ·  ENTER / ESC"
        );
        // field_menu/ui/spells_page.rs: a dash.
        assert_eq!(
            keyboard(
                &style("  -  ", "", "/", " / "),
                &[(&[Action(Confirm)], "CAST ON PARTY")]
            ),
            "ENTER  -  CAST ON PARTY"
        );
        // battle/reward_modal.rs: a key with no label.
        assert_eq!(
            keyboard(&FIELD_MENU_HINTS, &[(&[Action(Confirm)], "")]),
            "ENTER"
        );
        // battle/ui.rs and field_menu/ui/items_page.rs: a key inside a sentence.
        let bindings = InputBindings::default();
        let key =
            |keys: &[HintKey]| hint_keys_text(&bindings, Device::Keyboard, &FIELD_MENU_HINTS, keys);
        assert_eq!(
            format!("Choose a target. {} cancels.", key(&[Action(Back)])),
            "Choose a target. ESC cancels."
        );
        assert_eq!(
            format!("Choose another pouch with {}.", key(&[UP_DOWN])),
            "Choose another pouch with UP/DOWN."
        );
    }

    #[test]
    fn a_rebound_confirm_shows_its_new_key() {
        let segments: &[HintSegment<'_>] =
            &[(&[Action(Confirm)], "SAVE"), (&[Action(Back)], "BACK")];
        let mut bindings = InputBindings::default();
        bindings
            .set_keys(BindingTarget::Menu(Confirm), vec![KeyCode::KeyG])
            .expect("a nonempty row");
        assert_eq!(
            key_hint(&bindings, Device::Keyboard, &FIELD_MENU_HINTS, segments),
            "G   SAVE      ESC   BACK"
        );

        // Taking Enter for Back leaves Confirm with Space first; Back keeps Escape first.
        let mut bindings = InputBindings::default();
        bindings
            .bind_key(BindingTarget::Menu(Back), KeyCode::Enter)
            .expect("Confirm can spare Enter");
        assert_eq!(
            key_hint(&bindings, Device::Keyboard, &FIELD_MENU_HINTS, segments),
            "SPACE   SAVE      ESC   BACK"
        );
    }

    #[test]
    fn a_key_added_beside_the_defaults_leaves_the_text_alone() {
        let mut bindings = InputBindings::default();
        bindings
            .bind_key(BindingTarget::Menu(Confirm), KeyCode::KeyG)
            .expect("G is free");
        assert_eq!(
            key_hint(
                &bindings,
                Device::Keyboard,
                &FIELD_MENU_HINTS,
                &[(&[Action(Confirm)], "SAVE")]
            ),
            "ENTER   SAVE"
        );
    }

    #[test]
    fn a_rebound_half_of_a_pair_changes_only_that_half() {
        let mut bindings = InputBindings::default();
        bindings
            .set_keys(BindingTarget::Menu(Up), vec![KeyCode::KeyW])
            .expect("a nonempty row");
        bindings
            .set_keys(BindingTarget::Menu(Right), vec![KeyCode::NumpadEnter])
            .expect("a nonempty row");
        let spaced = style(" ", "    ", " / ", " / ");
        assert_eq!(
            key_hint(
                &bindings,
                Device::Keyboard,
                &spaced,
                &[(&[UP_DOWN], "Quantity ±1"), (&[LEFT_RIGHT], "Quantity ±10")],
            ),
            "W / DOWN Quantity ±1    LEFT / NUMPAD ENTER Quantity ±10"
        );
    }

    #[test]
    fn an_unbound_action_is_dropped_or_named_and_never_leaves_a_hole() {
        let mut bindings = InputBindings::default();
        bindings.clear_for_test(BindingTarget::Menu(Confirm));
        bindings.clear_for_test(BindingTarget::Menu(Up));
        let render = |bindings: &InputBindings, segments: &[HintSegment<'_>]| {
            key_hint(bindings, Device::Keyboard, &FIELD_MENU_HINTS, segments)
        };

        assert_eq!(
            render(
                &bindings,
                &[(&[Action(Confirm)], "SAVE"), (&[Action(Back)], "BACK")]
            ),
            "UNBOUND   SAVE      ESC   BACK"
        );
        assert_eq!(
            render(&bindings, &[(&[UP_DOWN], "SELECT")]),
            "DOWN   SELECT",
            "the bound half of a pair stands alone"
        );
        assert_eq!(
            render(&bindings, &[(&[Action(Confirm), Fixed("Y")], "OVERWRITE")]),
            "Y   OVERWRITE",
            "an unbound alternative is dropped"
        );

        bindings.clear_for_test(BindingTarget::Menu(Down));
        assert_eq!(
            render(&bindings, &[(&[UP_DOWN], "SELECT")]),
            "UNBOUND   SELECT"
        );
    }

    #[test]
    fn the_gamepad_column_names_the_first_bound_button() {
        assert_eq!(
            key_hint(
                &InputBindings::default(),
                Device::Gamepad,
                &FIELD_MENU_HINTS,
                &[
                    (&[UP_DOWN], "SELECT SLOT"),
                    (&[Action(Confirm), Fixed("Y")], "SAVE"),
                    (&[Action(Back)], "BACK"),
                ],
            ),
            "DPAD UP/DPAD DOWN   SELECT SLOT      SOUTH / Y   SAVE      EAST   BACK"
        );
    }

    #[test]
    fn every_bindable_input_has_a_nonempty_hint_name() {
        for key in ALL_BINDABLE_KEYS {
            let name = keyboard_hint_name(*key).expect("a bindable key has a name");
            assert!(!name.is_empty() && name == name.to_uppercase(), "{name}");
        }
        for button in ALL_BINDABLE_GAMEPAD_BUTTONS {
            let name = gamepad_hint_name(*button).expect("a bindable button has a name");
            assert!(!name.is_empty() && name == name.to_uppercase(), "{name}");
        }
        assert_eq!(
            keyboard_hint_name(KeyCode::NumpadEnter).unwrap(),
            "NUMPAD ENTER"
        );
        assert_eq!(
            keyboard_hint_name(KeyCode::ShiftLeft).unwrap(),
            "LEFT SHIFT"
        );
        assert_eq!(keyboard_hint_name(KeyCode::Numpad0).unwrap(), "NUMPAD 0");
        assert_eq!(keyboard_hint_name(KeyCode::F12).unwrap(), "F12");
        assert_eq!(keyboard_hint_name(KeyCode::MediaPlayPause), None);
    }
}
