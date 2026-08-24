use super::*;

pub(in crate::field_menu) fn screen_title(state: &FieldMenuState) -> &'static str {
    match state.screen {
        FieldMenuScreen::Main => "Field Menu",
        FieldMenuScreen::Status => "Status",
        FieldMenuScreen::Items => "Items",
        FieldMenuScreen::Equipment => "Equipment",
        FieldMenuScreen::Spells => "Spells",
        FieldMenuScreen::Quests => "Quest Board",
        FieldMenuScreen::Save => "Save Game",
    }
}

pub(in crate::field_menu) fn render_body(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    saves: &SaveSlotCatalog,
) -> String {
    if catalog.status() == CatalogStatus::Loading {
        return "Loading class and item catalogs...".to_owned();
    }
    if catalog.status() == CatalogStatus::Failed {
        return format!(
            "Catalog load failed:\n{}",
            catalog.failure().unwrap_or("unknown failure")
        );
    }
    let mut text = match state.screen {
        FieldMenuScreen::Main => render_main(state, game),
        FieldMenuScreen::Status => render_status(state, game, catalog),
        FieldMenuScreen::Items => render_items(state, game, catalog),
        FieldMenuScreen::Equipment => render_equipment(state, game, catalog),
        FieldMenuScreen::Spells => render_spells(state, game, catalog),
        FieldMenuScreen::Quests => render_quests(state, game, catalog),
        FieldMenuScreen::Save => render_save(state, saves),
    };
    if !state.message.is_empty() {
        text.push_str("\n\n");
        text.push_str(&state.message);
    }
    text
}

pub(in crate::field_menu) fn render_quests(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let quests = catalog.quests();
    let rows = quests
        .iter()
        .enumerate()
        .map(|(index, quest)| {
            let status = match quest_status(quest, game.flags()) {
                QuestStatus::Inactive => "INACTIVE",
                QuestStatus::Active => "ACTIVE",
                QuestStatus::Completed => "COMPLETE",
            };
            format!(
                "{} [{:<8}] {:<30}  {}",
                if index == state.selected { ">" } else { " " },
                status,
                quest.name,
                quest.location
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let detail = quests
        .get(state.selected)
        .map(|quest| format!("{}\n\n{}", quest.location, quest.description))
        .unwrap_or_else(|| "No quests are registered.".to_owned());
    format!("{rows}\n\n{detail}")
}

pub(in crate::field_menu) fn render_main(state: &FieldMenuState, game: &GameState) -> String {
    let commands = MAIN_COMMANDS
        .iter()
        .enumerate()
        .map(|(index, command)| {
            let cursor = if index == state.selected { ">" } else { " " };
            format!("{cursor} {}", command.label)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let party = game
        .party()
        .members()
        .map(|member| {
            format!(
                "{:<12} Lv {:>2}  HP {:>3}/{:<3}  MP {:>3}/{:<3}  {:?}",
                member.name(),
                member.level(),
                member.health(),
                member.max_health(),
                member.mana(),
                member.max_mana(),
                member.row()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{commands}\n\nParty                                      GP {:>8}\n{party}",
        game.repository().gp()
    )
}

pub(in crate::field_menu) fn render_status(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let Some(member) = member_at(game, state.member_index) else {
        return "No party members.".to_owned();
    };
    let stats = derived_stats(member, catalog);
    let statuses = member
        .status_effects()
        .map(|status| format!("{status:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let equipment = EquipmentSlot::ALL
        .into_iter()
        .map(|slot| {
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("(Empty)", item_name);
            format!("  {:<10} {name}", slot.as_str())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let abilities = learned_field_abilities(member, game, catalog)
        .into_iter()
        .map(|ability| ability.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Member {}/{}  {} — {}  Lv {}  {:?}\nHP {}/{}    MP {}/{}    EXP {}\n\nBase     STR {:>3}  DEX {:>3}  CON {:>3}  INT {:>3}\nDerived  STR {:>3}  DEX {:>3}  CON {:>3}  INT {:>3}\n\nEquipment\n{}\n\nField abilities: {}\nStatus effects: {}",
        state.member_index + 1,
        game.party().len(),
        member.name(),
        member.class_id(),
        member.level(),
        member.row(),
        member.health(),
        member.max_health(),
        member.mana(),
        member.max_mana(),
        member.experience(),
        member.stats().strength(),
        member.stats().dexterity(),
        member.stats().constitution(),
        member.stats().intelligence(),
        stats.strength,
        stats.dexterity,
        stats.constitution,
        stats.intelligence,
        equipment,
        if abilities.is_empty() {
            "None"
        } else {
            &abilities
        },
        if statuses.is_empty() {
            "None"
        } else {
            &statuses
        }
    )
}

pub(in crate::field_menu) fn render_items(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let tabs = InventoryTab::ALL
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            if index == state.tab_index {
                format!("[{}]", tab.label())
            } else {
                tab.label().to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("  ");
    let ids = inventory_ids(game, catalog, InventoryTab::ALL[state.tab_index]);
    let page = inventory_page_range(ids.len(), state.selected);
    let rows = ids
        .iter()
        .enumerate()
        .skip(page.start)
        .take(page.len())
        .map(|(index, id)| {
            let cursor = if state.mode == FieldMenuMode::Browse && index == state.selected {
                ">"
            } else {
                " "
            };
            let item = catalog.item(id).expect("filtered catalog item");
            format!(
                "{cursor} {:<30} x{:>3}",
                item_name(item),
                game.repository().item_count(id)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let detail = state
        .pending_id
        .as_deref()
        .or_else(|| ids.get(state.selected).copied())
        .and_then(|id| catalog.item(id))
        .map(item_description)
        .unwrap_or("No items in this tab.");
    let overlay = match state.mode {
        FieldMenuMode::ItemActions => ["Use", "Discard", "Hide"]
            .iter()
            .enumerate()
            .map(|(index, label)| {
                format!(
                    "{} {label}",
                    if index == state.selected { ">" } else { " " }
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        FieldMenuMode::DiscardQuantity => format!("Discard how many?  {}", state.quantity),
        FieldMenuMode::ItemTarget => render_targets(state, game),
        _ => String::new(),
    };
    format!(
        "{tabs}\n\n{}\n\n{}\n{}",
        if rows.is_empty() { "(empty)" } else { &rows },
        detail,
        overlay
    )
}

pub(in crate::field_menu) fn render_equipment(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let Some(member) = member_at(game, state.member_index) else {
        return "No party members.".to_owned();
    };
    let slot_index = state_slot_index(state);
    let slots = EquipmentSlot::ALL
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            let cursor = if state.mode == FieldMenuMode::Browse && index == state.selected {
                ">"
            } else {
                " "
            };
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("(Empty)", item_name);
            format!("{cursor} {:<10} {name}", slot.as_str())
        })
        .collect::<Vec<_>>()
        .join("\n");
    if state.mode != FieldMenuMode::EquipmentPicker {
        return format!(
            "Member {}/{}  {} — {}\n\n{slots}",
            state.member_index + 1,
            game.party().len(),
            member.name(),
            member.class_id()
        );
    }
    let slot = EquipmentSlot::ALL[slot_index];
    let candidates = equipment_candidates(game, catalog, slot);
    let rows = std::iter::once(None)
        .chain(candidates.iter().map(|id| Some(id.as_str())))
        .enumerate()
        .map(|(index, id)| {
            let cursor = if index == state.selected { ">" } else { " " };
            match id {
                None => format!("{cursor} (Unequip)"),
                Some(id) => {
                    let item = catalog.item(id).expect("candidate exists");
                    let blocked = can_equip(member, item, catalog)
                        .err()
                        .map(|error| format!("  [{}]", error))
                        .unwrap_or_default();
                    format!("{cursor} {}{blocked}", item_name(item))
                }
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let selected_id = state
        .selected
        .checked_sub(1)
        .and_then(|index| candidates.get(index).map(String::as_str));
    let before = derived_stats(member, catalog);
    let after = preview_stats(member, catalog, slot, selected_id);
    format!(
        "Member {}/{}  {} — {}\n\n{slots}\n\nChoose {}\n{}\n\nPreview  STR {}->{}, DEX {}->{}, CON {}->{}, INT {}->{}",
        state.member_index + 1,
        game.party().len(),
        member.name(),
        member.class_id(),
        slot.as_str(),
        rows,
        before.strength,
        after.strength,
        before.dexterity,
        after.dexterity,
        before.constitution,
        after.constitution,
        before.intelligence,
        after.intelligence
    )
}

pub(in crate::field_menu) fn render_spells(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let Some(member) = member_at(game, state.member_index) else {
        return "No party members.".to_owned();
    };
    let abilities = learned_field_abilities(member, game, catalog);
    let rows = abilities
        .iter()
        .enumerate()
        .map(|(index, ability)| {
            format!(
                "{} {:<24} MP {:>3}  {}",
                if state.mode == FieldMenuMode::Browse && index == state.selected {
                    ">"
                } else {
                    " "
                },
                ability.name,
                ability.mp_cost,
                ability.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let overlay = match state.mode {
        FieldMenuMode::SpellTarget => render_targets(state, game),
        FieldMenuMode::TeleportPicker => catalog
            .eligible_warp_destinations(game.map())
            .iter()
            .enumerate()
            .map(|(index, destination)| {
                format!(
                    "{} {}",
                    if index == state.selected { ">" } else { " " },
                    destination.name
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    format!(
        "Member {}/{}  {} — MP {}/{}\n\n{}\n\n{}",
        state.member_index + 1,
        game.party().len(),
        member.name(),
        member.mana(),
        member.max_mana(),
        if rows.is_empty() {
            "(No learned field abilities)"
        } else {
            &rows
        },
        overlay
    )
}

pub(in crate::field_menu) fn render_targets(state: &FieldMenuState, game: &GameState) -> String {
    game.party()
        .members()
        .enumerate()
        .map(|(index, member)| {
            format!(
                "{} {:<12} HP {}/{}  MP {}/{}",
                if index == state.selected { ">" } else { " " },
                member.name(),
                member.health(),
                member.max_health(),
                member.mana(),
                member.max_mana()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(in crate::field_menu) fn render_save(
    state: &FieldMenuState,
    saves: &SaveSlotCatalog,
) -> String {
    let page_start =
        ((state.selected.saturating_sub(FIRST_PLAYER_SLOT)) / 7) * 7 + FIRST_PLAYER_SLOT;
    let rows = saves
        .slots()
        .iter()
        .skip(page_start)
        .take(7)
        .map(|slot| {
            let cursor = if slot.index == state.selected {
                ">"
            } else {
                " "
            };
            match (&slot.state, &slot.metadata) {
                (SaveSlotState::Empty, _) => {
                    format!("{cursor} {:<8} --- Empty ---", slot.label())
                }
                (SaveSlotState::Valid, Some(metadata)) => format!(
                    "{cursor} {:<8} {} Lv{}  {}  {}",
                    slot.label(),
                    metadata.protagonist_name,
                    metadata.protagonist_level,
                    crate::playtime::Playtime::format(metadata.playtime_seconds),
                    metadata.location,
                ),
                (SaveSlotState::Corrupt(_), _) => {
                    format!("{cursor} {:<8} [CORRUPT]", slot.label())
                }
                (SaveSlotState::Incompatible(_), _) => {
                    format!("{cursor} {:<8} [INCOMPATIBLE]", slot.label())
                }
                _ => format!("{cursor} {:<8} [INVALID METADATA]", slot.label()),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if rows.is_empty() {
        "Discovering native save slots...".to_owned()
    } else {
        rows
    }
}

pub(in crate::field_menu) fn render_hint(state: &FieldMenuState) -> String {
    match (state.screen, state.mode) {
        (FieldMenuScreen::Main, FieldMenuMode::QuitConfirm) => {
            "Y/ENTER exit to desktop  N/ESC cancel"
        }
        (FieldMenuScreen::Main, _) => "UP/DOWN choose  ENTER open  M/ESC close",
        (FieldMenuScreen::Status, _) if state.status_page == StatusPage::Roster => {
            "UP/DOWN member  ENTER stats  ESC back  M close"
        }
        (FieldMenuScreen::Status, _) => "UP/DOWN action  ESC portrait  M close",
        (FieldMenuScreen::Items, FieldMenuMode::Browse) => {
            "LEFT/RIGHT tab  UP/DOWN item  ENTER actions  ESC back"
        }
        (FieldMenuScreen::Items, FieldMenuMode::DiscardQuantity) => {
            "UP/DOWN quantity  LEFT one  RIGHT whole stack  ENTER discard  ESC cancel"
        }
        (FieldMenuScreen::Equipment, FieldMenuMode::Browse) => {
            "LEFT/RIGHT member  UP/DOWN slot  ENTER choose  ESC back"
        }
        (FieldMenuScreen::Spells, FieldMenuMode::Browse) => {
            "LEFT/RIGHT member  UP/DOWN spell  ENTER cast  ESC back"
        }
        (FieldMenuScreen::Quests, _) => "UP/DOWN quest  ESC back  M close",
        (FieldMenuScreen::Save, FieldMenuMode::Browse) => {
            "UP/DOWN slot  ENTER save  ESC back  M close"
        }
        (FieldMenuScreen::Save, FieldMenuMode::SaveConfirm) => "Y/ENTER overwrite  N/ESC cancel",
        (_, FieldMenuMode::ItemTarget | FieldMenuMode::SpellTarget) => {
            "UP/DOWN target  ENTER confirm  ESC cancel"
        }
        (_, FieldMenuMode::TeleportPicker) => "UP/DOWN destination  ENTER teleport  ESC cancel",
        _ => "UP/DOWN choose  ENTER confirm  ESC cancel  M close",
    }
    .to_owned()
}
