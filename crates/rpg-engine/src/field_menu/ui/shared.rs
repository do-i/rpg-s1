use super::*;

/// Keys the field menu reads through the binding table. The arrow keys that move a column
/// sideways are read raw (`input.horizontal`), so those hints use [`HintKey::Fixed`].
pub(in crate::field_menu) const UP_DOWN: HintKey = HintKey::Pair(AppAction::Up, AppAction::Down);
pub(in crate::field_menu) const DOWN: HintKey = HintKey::Action(AppAction::Down);
pub(in crate::field_menu) const CONFIRM: HintKey = HintKey::Action(AppAction::Confirm);
pub(in crate::field_menu) const BACK: HintKey = HintKey::Action(AppAction::Back);
/// Left and right as bound actions, which page the battle log and pick a sprite.
pub(in crate::field_menu) const PAGE: HintKey = HintKey::Pair(AppAction::Left, AppAction::Right);
pub(in crate::field_menu) const LEFT: HintKey = HintKey::Fixed("LEFT");
pub(in crate::field_menu) const RIGHT: HintKey = HintKey::Fixed("RIGHT");
pub(in crate::field_menu) const LEFT_RIGHT: HintKey = HintKey::Fixed("LEFT/RIGHT");

/// The compact fallback footers: `UP/DOWN choose  ENTER open  M/ESC close`.
pub(in crate::field_menu) const FALLBACK_HINTS: HintStyle = HintStyle {
    key_gap: " ",
    segment_gap: "  ",
    pair: "/",
    either: "/",
    label_first: false,
};

/// The status page footer: `Q/E  CHARACTER    LEFT/RIGHT  SELECT    TAB / ENTER  POSITION`.
pub(in crate::field_menu) const STATUS_HINTS: HintStyle = HintStyle {
    key_gap: "  ",
    segment_gap: "    ",
    pair: "/",
    either: " / ",
    label_first: false,
};

/// The switch overlay's one-line footer: `UP/DOWN CHARACTER   LEFT/RIGHT SPRITE   ENTER APPLY`.
pub(in crate::field_menu) const MODAL_HINTS: HintStyle = HintStyle {
    key_gap: " ",
    segment_gap: "   ",
    pair: "/",
    either: " / ",
    label_first: false,
};

/// Notes inside item modals: `LEFT one    RIGHT whole stack`, `Y/ENTER use    N/ESC cancel`.
pub(in crate::field_menu) const NOTE_HINTS: HintStyle = HintStyle {
    key_gap: " ",
    segment_gap: "    ",
    pair: "/",
    either: "/",
    label_first: false,
};

/// A lone key and what it does inside a panel: `ENTER  -  ACTIONS`.
pub(in crate::field_menu) const DASH_HINTS: HintStyle = HintStyle {
    key_gap: "  -  ",
    segment_gap: "",
    pair: "/",
    either: " / ",
    label_first: false,
};

/// Renders a field-menu hint from the live keyboard bindings.
pub(in crate::field_menu) fn menu_hint(
    bindings: &InputBindings,
    style: &HintStyle,
    segments: &[HintSegment<'_>],
) -> String {
    key_hint(bindings, Device::Keyboard, style, segments)
}

/// The keys of one hint written inside a sentence.
pub(in crate::field_menu) fn menu_hint_keys(
    bindings: &InputBindings,
    style: &HintStyle,
    keys: &[HintKey],
) -> String {
    hint_keys_text(bindings, Device::Keyboard, style, keys)
}

/// What a page system should do this frame, after the shared lifecycle checks.
pub(in crate::field_menu) enum PageSync {
    Skip,
    Rebuild(PageRebuild),
}

pub(in crate::field_menu) struct PageRebuild {
    pub menu_root: Entity,
    pub font: Handle<Font>,
}

/// The opening every field-menu page shares: despawn when hidden, find the menu root, rebuild
/// only when the page is missing or `changed`, despawn the old page, and load the font.
#[expect(
    clippy::too_many_arguments,
    reason = "the shared page opening takes the same Bevy parameters each page already holds"
)]
pub(in crate::field_menu) fn begin_page_sync<P: Component>(
    commands: &mut Commands,
    visible: bool,
    changed: bool,
    menu_roots: &Query<Entity, With<FieldMenuRoot>>,
    pages: &Query<Entity, With<P>>,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    inventory: &ScenarioInventory,
) -> PageSync {
    if !visible {
        for entity in pages {
            commands.entity(entity).despawn();
        }
        return PageSync::Skip;
    }
    let Ok(menu_root) = menu_roots.single() else {
        return PageSync::Skip;
    };
    if !(pages.is_empty() || changed) {
        return PageSync::Skip;
    }
    for entity in pages {
        commands.entity(entity).despawn();
    }
    let Some(font) = scenario_font(asset_server, root, inventory) else {
        return PageSync::Skip;
    };
    PageSync::Rebuild(PageRebuild { menu_root, font })
}

pub(in crate::field_menu) fn selected_inventory_index(
    state: &FieldMenuState,
    ids: &[&str],
) -> usize {
    if ids.is_empty() {
        return 0;
    }
    if state.selected_is_list_cursor() {
        return state.selected.min(ids.len() - 1);
    }
    state
        .pending_id
        .as_deref()
        .and_then(|pending| ids.iter().position(|id| *id == pending))
        .unwrap_or(0)
}

pub(in crate::field_menu) fn selected_inventory_id<'a>(
    state: &'a FieldMenuState,
    ids: &'a [&'a str],
    list_index: usize,
) -> Option<&'a str> {
    if state.selected_is_list_cursor() {
        ids.get(list_index).copied()
    } else {
        state.pending_id.as_deref()
    }
}

pub(in crate::field_menu) fn item_kind_label(item: &ItemDefinition) -> &'static str {
    match item {
        ItemDefinition::Consumable(_) => "CONSUMABLE",
        ItemDefinition::Material(_) => "MATERIAL",
        ItemDefinition::Key(_) => "KEY ITEM",
        ItemDefinition::MagicCore(_) => "MAGIC CORE",
        ItemDefinition::Weapon(_) => "WEAPON",
        ItemDefinition::Shield(_) => "SHIELD",
        ItemDefinition::Helmet(_) => "HELMET",
        ItemDefinition::Body(_) => "BODY ARMOR",
        ItemDefinition::Accessory(_) => "ACCESSORY",
    }
}

/// The type-driven tags the kind chip already says, so the tag chips can skip them.
pub(in crate::field_menu) fn item_kind_tags(item: &ItemDefinition) -> &'static [&'static str] {
    match item {
        ItemDefinition::Consumable(_) => &["consumable"],
        ItemDefinition::Material(_) => &["material"],
        ItemDefinition::Key(_) => &["key"],
        ItemDefinition::MagicCore(_) => &["magic_core"],
        ItemDefinition::Weapon(_) => &["equipment", "weapon"],
        ItemDefinition::Shield(_) => &["equipment", "shield"],
        ItemDefinition::Helmet(_) => &["equipment", "helmet"],
        ItemDefinition::Body(_) => &["equipment", "body"],
        ItemDefinition::Accessory(_) => &["equipment", "accessory"],
    }
}

pub(in crate::field_menu) fn item_kind_abbreviation(item: &ItemDefinition) -> &'static str {
    match item {
        ItemDefinition::Consumable(_) => "USE",
        ItemDefinition::Material(_) => "MAT",
        ItemDefinition::Key(_) => "KEY",
        ItemDefinition::MagicCore(_) => "CORE",
        ItemDefinition::Weapon(_) => "WPN",
        ItemDefinition::Shield(_) => "SHLD",
        ItemDefinition::Helmet(_) => "HELM",
        ItemDefinition::Body(_) => "BODY",
        ItemDefinition::Accessory(_) => "ACC",
    }
}

pub(in crate::field_menu) fn item_accent(item: &ItemDefinition) -> Color {
    match item {
        ItemDefinition::Consumable(_) => status_teal(),
        ItemDefinition::Material(_) => Color::srgb_u8(157, 139, 101),
        ItemDefinition::Key(_) => status_gold(),
        ItemDefinition::MagicCore(_) => status_violet(),
        ItemDefinition::Weapon(_) => status_ember(),
        ItemDefinition::Shield(_) => Color::srgb_u8(91, 143, 183),
        ItemDefinition::Helmet(_) => Color::srgb_u8(126, 151, 174),
        ItemDefinition::Body(_) => Color::srgb_u8(112, 126, 148),
        ItemDefinition::Accessory(_) => Color::srgb_u8(196, 116, 168),
    }
}

pub(in crate::field_menu) fn member_emblem(name: &str) -> String {
    let words = name.split_whitespace().collect::<Vec<_>>();
    let letters = if words.len() > 1 {
        words
            .iter()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
    } else {
        name.chars()
            .filter(|character| !character.is_whitespace())
            .take(2)
            .collect()
    };
    let emblem = letters.to_uppercase();
    if emblem.is_empty() {
        "?".to_owned()
    } else {
        emblem
    }
}

pub(in crate::field_menu) fn class_name<'a>(
    class_id: &'a str,
    catalog: &'a FieldMenuCatalog,
) -> &'a str {
    catalog
        .class(class_id)
        .map_or(class_id, |class| class.name.as_str())
}
