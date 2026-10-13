//! Items page actions: browsing the pouch, item actions, targeting, discards, tags, and the
//! show/hide manager.

use bevy::prelude::*;

use super::{
    input::MenuInput,
    member_id_at,
    state::{FieldMenuMode, FieldMenuState, wrapped, wrapped_or_zero, wrapped_quantity},
};
use crate::{
    engine_settings::EngineSettings,
    field_menu_domain::{
        CUSTOM_TAG_MAX_LENGTH, EDITABLE_SYSTEM_TAGS, FieldMenuCatalog, InventoryTab, custom_tags,
        discard_item, inventory_ids, normalize_custom_tag, targets_whole_party, use_field_item,
        use_field_item_on_party,
    },
    game_state::GameState,
    scenario_item::ItemDefinition,
    sfx_cue::MenuSfx,
};

/// Item-action rows, in the order the modal lists them.
pub(super) const ITEM_ACTIONS: [(&str, &str); 5] = [
    ("Use", "apply this item"),
    ("Discard", "remove from pouch"),
    ("Hide", "hide for this session"),
    ("Edit Tags", "curate and add tags"),
    ("Lock", "protect from discard and sale"),
];
pub(super) const ITEM_ACTION_TAGS: usize = 3;
pub(super) const ITEM_ACTION_LOCK: usize = 4;

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    game: &mut ResMut<'_, GameState>,
    input: &MenuInput<'_>,
    catalog: &FieldMenuCatalog,
    engine: &EngineSettings,
    menu_sfx: &mut MenuSfx<'_>,
) {
    match state.mode {
        FieldMenuMode::ItemAoeConfirm => {
            if input.keys.just_pressed(KeyCode::KeyN) {
                state.mode = FieldMenuMode::ItemActions;
                state.message.clear();
            } else if input.keys.just_pressed(KeyCode::KeyY) || input.confirm {
                let id = state
                    .pending_id
                    .clone()
                    .expect("a party-wide confirm requires an item");
                apply_party_item(state, game, catalog, &id);
            }
        }
        // Pouch column: Up/Down picks the tab, Right or Enter moves into its items.
        FieldMenuMode::Browse => {
            if let Some(delta) = input.vertical {
                state.tab_index = wrapped(state.tab_index, InventoryTab::ALL.len(), delta);
                state.selected = 0;
            }
            if input.keys.just_pressed(KeyCode::KeyH) {
                if manage_ids(game, catalog).is_empty() {
                    state.message = "The pouch is empty.".to_owned();
                } else {
                    state.leave_list(FieldMenuMode::ItemManage);
                }
            } else if input.horizontal == Some(1) || input.confirm {
                if inventory_ids(game, catalog, InventoryTab::ALL[state.tab_index]).is_empty() {
                    menu_sfx.blocked();
                    state.message = "This tab is empty.".to_owned();
                } else {
                    state.mode = FieldMenuMode::ItemList;
                }
            }
        }
        // Item column: Up/Down picks the item, Enter opens its actions, Left returns to the
        // pouch.
        FieldMenuMode::ItemList => {
            let ids = inventory_ids(game, catalog, InventoryTab::ALL[state.tab_index]);
            // Spending or hiding the last of a stack can leave the cursor past the end.
            state.selected = state.selected.min(ids.len().saturating_sub(1));
            if input.horizontal == Some(-1) {
                state.back();
                return;
            }
            if let Some(delta) = input.vertical {
                state.selected = wrapped_or_zero(state.selected, ids.len(), delta);
            }
            // Python opens the manage modal with M; here M closes the whole field menu, so the
            // show/hide manager takes H instead.
            if input.keys.just_pressed(KeyCode::KeyH) {
                if manage_ids(game, catalog).is_empty() {
                    state.message = "The pouch is empty.".to_owned();
                } else {
                    state.leave_list(FieldMenuMode::ItemManage);
                }
            } else if input.confirm {
                if let Some(id) = ids.get(state.selected) {
                    state.pending_id = Some((*id).to_owned());
                    state.leave_list(FieldMenuMode::ItemActions);
                } else {
                    state.message = "This tab is empty.".to_owned();
                }
            }
        }
        FieldMenuMode::ItemActions => {
            if let Some(delta) = input.vertical {
                state.selected = wrapped(state.selected, ITEM_ACTIONS.len(), delta);
            }
            if input.confirm {
                let id = state
                    .pending_id
                    .clone()
                    .expect("item action requires an item");
                match state.selected {
                    // A party-wide item never reaches the target picker: the source branches on
                    // the effect's target before offering one, and routing a Tent through the
                    // picker is what made it unusable -- every member was an invalid target.
                    0 if targets_whole_party(catalog, &id) => {
                        if engine.use_aoe_confirm {
                            state.mode = FieldMenuMode::ItemAoeConfirm;
                            state.selected = 0;
                        } else {
                            apply_party_item(state, game, catalog, &id);
                        }
                    }
                    0 if catalog.field_use(&id).is_some() => {
                        state.mode = FieldMenuMode::ItemTarget;
                        state.selected = 0;
                    }
                    0 => state.message = "That item cannot be used in the field.".to_owned(),
                    1 => {
                        if matches!(catalog.item(&id), Some(ItemDefinition::Key(_))) {
                            state.message = "Key items cannot be discarded.".to_owned();
                        } else if game.repository().is_locked(&id) {
                            state.message = "That item is locked.".to_owned();
                        } else {
                            state.mode = FieldMenuMode::DiscardQuantity;
                            state.selected = 0;
                            state.quantity = 1;
                        }
                    }
                    2 => {
                        game.repository_mut().set_hidden(id, true);
                        state.return_to_list();
                        state.message = "Hidden for this session.".to_owned();
                    }
                    ITEM_ACTION_TAGS => {
                        state.mode = FieldMenuMode::ItemTags;
                        state.selected = 0;
                    }
                    ITEM_ACTION_LOCK => toggle_item_lock(game, state, &id),
                    _ => unreachable!(),
                }
            }
        }
        FieldMenuMode::ItemTags => {
            let id = state
                .pending_id
                .clone()
                .expect("the tag editor requires an item");
            let rows = tag_editor_rows(game, &id);
            if let Some(delta) = input.vertical {
                state.selected = wrapped(state.selected, rows.len(), delta);
            }
            if input.confirm {
                match rows.get(state.selected) {
                    Some(TagEditorRow::New) => {
                        state.mode = FieldMenuMode::ItemNewTag;
                        state.text_input.clear();
                        state.message.clear();
                    }
                    Some(TagEditorRow::Tag(tag)) => {
                        let tag = tag.clone();
                        toggle_item_tag(game, state, &id, &tag);
                    }
                    None => {}
                }
            }
        }
        FieldMenuMode::ItemNewTag => {
            let id = state
                .pending_id
                .clone()
                .expect("the tag editor requires an item");
            for key in &input.typed {
                match key.key_code {
                    KeyCode::Backspace => {
                        state.text_input.pop();
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter if !key.repeat => {
                        commit_new_tag(game, state, &id);
                    }
                    KeyCode::Escape => {}
                    _ => {
                        if let Some(text) = key.text.as_deref() {
                            append_tag_text(&mut state.text_input, text);
                        }
                    }
                }
            }
        }
        FieldMenuMode::ItemManage => {
            let ids = manage_ids(game, catalog);
            if let Some(delta) = input.vertical {
                state.selected = wrapped_or_zero(state.selected, ids.len(), delta);
            }
            if input.keys.just_pressed(KeyCode::KeyH) {
                state.back();
            } else if input.confirm
                && let Some(id) = ids.get(state.selected).map(|id| (*id).to_owned())
            {
                let hidden = game.repository_mut().toggle_hidden(&id);
                state.message = if hidden {
                    "Hidden from the pouch.".to_owned()
                } else {
                    "Shown in the pouch again.".to_owned()
                };
            }
        }
        FieldMenuMode::DiscardQuantity => {
            let id = state
                .pending_id
                .as_deref()
                .expect("discard requires item")
                .to_owned();
            let max = game.repository().item_count(&id).max(1);
            if let Some(delta) = input.vertical {
                state.quantity = wrapped_quantity(state.quantity, max, delta);
            }
            if input.horizontal == Some(-1) {
                state.quantity = 1;
            }
            if input.horizontal == Some(1) {
                state.quantity = max;
            }
            if input.confirm {
                match discard_item(game, catalog, &id, state.quantity) {
                    Ok(()) => {
                        state.return_to_list();
                        state.message = "Items discarded.".to_owned();
                    }
                    Err(error) => state.message = error.to_string(),
                }
            }
        }
        FieldMenuMode::ItemTarget => {
            if let Some(delta) = input.vertical {
                state.selected = wrapped_or_zero(state.selected, game.party().len(), delta);
            }
            if input.confirm {
                let Some(target_id) = member_id_at(game, state.selected).map(str::to_owned) else {
                    return;
                };
                let item_id = state
                    .pending_id
                    .clone()
                    .expect("item targeting requires item");
                match use_field_item(game, catalog, &item_id, &target_id) {
                    Ok(changed) => {
                        state.return_to_list();
                        state.message = format!("Applied {changed} points/effects.");
                    }
                    Err(error) => state.message = error.to_string(),
                }
            }
        }
        _ => {}
    }
}

/// Spends a party-wide item and returns the menu to browsing, from either route into it.
pub(super) fn apply_party_item(
    state: &mut FieldMenuState,
    game: &mut GameState,
    catalog: &FieldMenuCatalog,
    item_id: &str,
) {
    match use_field_item_on_party(game, catalog, item_id) {
        Ok(changed) => {
            state.return_to_list();
            state.message = format!("Applied {changed} points/effects to the party.");
        }
        Err(error) => {
            state.mode = FieldMenuMode::ItemActions;
            state.message = error.to_string();
        }
    }
}

/// One row of the tag editor: a toggleable tag, or the prompt that opens free-text entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum TagEditorRow {
    Tag(String),
    New,
}

/// The editor's rows for one item: the curatorial set, then that item's own tags, then `New tag`.
///
/// Ports `item_scene._editor_rows`. Type-driven tags are catalog-owned and never listed, so a row
/// here is always something the player may add or drop.
pub(super) fn tag_editor_rows(game: &GameState, item_id: &str) -> Vec<TagEditorRow> {
    let mut rows = EDITABLE_SYSTEM_TAGS
        .into_iter()
        .map(|tag| TagEditorRow::Tag(tag.to_owned()))
        .collect::<Vec<_>>();
    rows.extend(
        custom_tags(game, item_id)
            .into_iter()
            .map(|tag| TagEditorRow::Tag(tag.to_owned())),
    );
    rows.push(TagEditorRow::New);
    rows
}

/// Adds a tag the item lacks, or removes one it has. Ports `item_scene._activate_editor_row`.
pub(super) fn toggle_item_tag(
    game: &mut GameState,
    state: &mut FieldMenuState,
    item_id: &str,
    tag: &str,
) {
    if game.repository().item_tags(item_id).any(|held| held == tag) {
        game.repository_mut().remove_tag(item_id, tag);
        state.message = format!("Removed `{tag}`.");
        return;
    }
    let cap = game.repository().max_tags_per_item();
    if game.repository_mut().add_tags(item_id, [tag]).is_err() {
        state.message = format!("Max tags ({cap}) reached.");
    } else {
        state.message = format!("Added `{tag}`.");
    }
}

/// Toggles the repository's persistent discard/sale protection for one owned stack.
pub(super) fn toggle_item_lock(game: &mut GameState, state: &mut FieldMenuState, item_id: &str) {
    let locked = !game.repository().is_locked(item_id);
    game.repository_mut().set_locked(item_id, locked);
    state.message = if locked {
        "Locked against discarding and selling.".to_owned()
    } else {
        "Unlocked for discarding and selling.".to_owned()
    };
}

/// Returns the live label for an item action; lock protection is the only stateful row.
pub(super) fn item_action_row(
    index: usize,
    game: &GameState,
    item_id: &str,
) -> (&'static str, &'static str) {
    if index == ITEM_ACTION_LOCK && game.repository().is_locked(item_id) {
        ("Unlock", "allow discard and sale")
    } else {
        ITEM_ACTIONS[index]
    }
}

/// Validates and stores the drafted custom tag, keeping the prompt open on rejection.
///
/// Ports `item_scene._commit_new_tag`, which distinguishes an invalid tag, a duplicate, and the
/// per-item cap so the player learns which one blocked them.
pub(super) fn commit_new_tag(game: &mut GameState, state: &mut FieldMenuState, item_id: &str) {
    let Some(tag) = normalize_custom_tag(&state.text_input) else {
        state.message = "Invalid tag.".to_owned();
        return;
    };
    if game.repository().item_tags(item_id).any(|held| held == tag) {
        state.message = "Tag already added.".to_owned();
        return;
    }
    let cap = game.repository().max_tags_per_item();
    if game
        .repository_mut()
        .add_tags(item_id, [tag.as_str()])
        .is_err()
    {
        state.message = format!("Max tags ({cap}) reached.");
        return;
    }
    state.mode = FieldMenuMode::ItemTags;
    state.text_input.clear();
    state.message = format!("Added `{tag}`.");
}

/// Appends one scalar at a time so a multi-scalar platform text event cannot cross the cap.
pub(super) fn append_tag_text(draft: &mut String, text: &str) {
    let remaining = CUSTOM_TAG_MAX_LENGTH.saturating_sub(draft.chars().count());
    draft.extend(
        text.chars()
            .filter(|character| !character.is_control())
            .take(remaining),
    );
}

/// Every owned item the manage modal lists, hidden ones included.
///
/// Ports `item_scene._manage_entries`, which sorts by id and deliberately ignores the tab filter
/// so a hidden item is always reachable again.
pub(super) fn manage_ids<'a>(game: &'a GameState, catalog: &FieldMenuCatalog) -> Vec<&'a str> {
    let mut ids = game
        .repository()
        .item_counts()
        .map(|(id, _)| id)
        .filter(|id| catalog.item(id).is_some())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_data::tests::fixture_game;

    #[test]
    fn the_tag_editor_lists_the_curatorial_set_then_custom_tags_then_the_new_prompt() {
        let mut game = fixture_game();
        game.repository_mut().add_tags("potion", ["mine"]).unwrap();

        let rows = tag_editor_rows(&game, "potion");

        assert_eq!(
            rows,
            vec![
                TagEditorRow::Tag("rare".to_owned()),
                TagEditorRow::Tag("sell_soon".to_owned()),
                TagEditorRow::Tag("favorite".to_owned()),
                TagEditorRow::Tag("mine".to_owned()),
                TagEditorRow::New,
            ]
        );
        // Type-driven catalog tags stay out of the editor: the player does not own them.
        game.repository_mut()
            .add_tags("potion", ["consumable"])
            .unwrap();
        assert!(
            !tag_editor_rows(&game, "potion").contains(&TagEditorRow::Tag("consumable".to_owned()))
        );
    }

    #[test]
    fn toggling_a_tag_adds_it_once_and_removes_it_again() {
        let mut game = fixture_game();
        let mut state = FieldMenuState::default();

        toggle_item_tag(&mut game, &mut state, "potion", "favorite");
        assert!(
            game.repository()
                .item_tags("potion")
                .any(|tag| tag == "favorite")
        );
        assert!(state.message.contains("Added"));

        toggle_item_tag(&mut game, &mut state, "potion", "favorite");
        assert!(
            !game
                .repository()
                .item_tags("potion")
                .any(|tag| tag == "favorite")
        );
        assert!(state.message.contains("Removed"));
    }

    #[test]
    fn toggling_a_lock_updates_protection_and_the_live_action_label() {
        let mut game = fixture_game();
        let mut state = FieldMenuState::default();

        assert!(game.repository().is_locked("potion"));
        assert_eq!(
            item_action_row(ITEM_ACTION_LOCK, &game, "potion").0,
            "Unlock"
        );
        toggle_item_lock(&mut game, &mut state, "potion");
        assert!(!game.repository().is_locked("potion"));
        assert_eq!(state.message, "Unlocked for discarding and selling.");
        assert_eq!(item_action_row(ITEM_ACTION_LOCK, &game, "potion").0, "Lock");

        toggle_item_lock(&mut game, &mut state, "potion");
        assert!(game.repository().is_locked("potion"));
        assert_eq!(state.message, "Locked against discarding and selling.");
    }

    #[test]
    fn a_custom_tag_is_normalized_and_rejected_reasons_are_distinguished() {
        assert_eq!(
            normalize_custom_tag("  Sell Later "),
            Some("sell_later".to_owned())
        );
        assert_eq!(normalize_custom_tag("KEEP"), Some("keep".to_owned()));
        assert!(normalize_custom_tag("   ").is_none());
        assert!(normalize_custom_tag("boss-drop").is_none());
        assert!(normalize_custom_tag(&"a".repeat(CUSTOM_TAG_MAX_LENGTH + 1)).is_none());

        let mut game = fixture_game();
        let mut state = FieldMenuState {
            mode: FieldMenuMode::ItemNewTag,
            text_input: "boss-drop".to_owned(),
            ..default()
        };

        commit_new_tag(&mut game, &mut state, "potion");
        assert_eq!(
            state.mode,
            FieldMenuMode::ItemNewTag,
            "an invalid tag keeps the prompt open"
        );
        assert_eq!(state.message, "Invalid tag.");

        state.text_input = "Boss Drop".to_owned();
        commit_new_tag(&mut game, &mut state, "potion");
        assert_eq!(state.mode, FieldMenuMode::ItemTags);
        assert!(state.text_input.is_empty());
        assert!(
            game.repository()
                .item_tags("potion")
                .any(|tag| tag == "boss_drop")
        );

        state.mode = FieldMenuMode::ItemNewTag;
        state.text_input = "boss_drop".to_owned();
        commit_new_tag(&mut game, &mut state, "potion");
        assert_eq!(state.message, "Tag already added.");
    }

    #[test]
    fn the_tag_draft_never_exceeds_the_cap_even_from_one_multi_scalar_event() {
        let mut draft = String::new();
        append_tag_text(&mut draft, &"x".repeat(CUSTOM_TAG_MAX_LENGTH + 8));
        assert_eq!(draft.chars().count(), CUSTOM_TAG_MAX_LENGTH);

        let mut short = "ab".to_owned();
        append_tag_text(&mut short, "\u{7}c");
        assert_eq!(short, "abc", "control scalars are dropped, not stored");
    }

    #[test]
    fn the_manage_list_keeps_hidden_items_that_the_pouch_filters_out() {
        // The fixture already hides its potion, which is exactly the state the manager exists for.
        let mut game = fixture_game();
        let catalog = crate::field_menu_domain::tests::catalog();

        assert!(game.repository().is_hidden("potion"));
        assert!(!inventory_ids(&game, &catalog, InventoryTab::All).contains(&"potion"));
        assert!(
            manage_ids(&game, &catalog).contains(&"potion"),
            "a hidden item must stay reachable so it can be shown again"
        );

        assert!(!game.repository_mut().toggle_hidden("potion"));
        assert!(inventory_ids(&game, &catalog, InventoryTab::All).contains(&"potion"));
    }
}
