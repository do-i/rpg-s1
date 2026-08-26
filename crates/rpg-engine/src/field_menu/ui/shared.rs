use super::*;

pub(in crate::field_menu) fn spawn_meter(
    parent: &mut ChildSpawnerCommands<'_>,
    label: &str,
    value: u32,
    maximum: u32,
    font: &Handle<Font>,
    fill: Color,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            ..default()
        })
        .with_children(|meter| {
            meter
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|label_row| {
                    spawn_status_text(label_row, label, font, 13.0, status_muted());
                    spawn_status_text(
                        label_row,
                        format!("{value} / {maximum}"),
                        font,
                        13.0,
                        status_ink(),
                    );
                });
            meter
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(8),
                        border_radius: BorderRadius::all(px(4)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(17, 17, 22)),
                ))
                .with_children(|track| {
                    track.spawn((
                        Node {
                            width: percent(meter_percent(value, maximum)),
                            height: percent(100),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(fill),
                    ));
                });
        });
}

pub(in crate::field_menu) fn meter_percent(value: u32, maximum: u32) -> f32 {
    if maximum == 0 {
        0.0
    } else {
        (value as f32 / maximum as f32 * 100.0).clamp(0.0, 100.0)
    }
}

pub(in crate::field_menu) fn selected_inventory_index(
    state: &FieldMenuState,
    ids: &[&str],
) -> usize {
    if ids.is_empty() {
        return 0;
    }
    if state.mode == FieldMenuMode::Browse {
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
    if state.mode == FieldMenuMode::Browse {
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

pub(in crate::field_menu) fn status_slot_label(slot: EquipmentSlot) -> &'static str {
    match slot {
        EquipmentSlot::Weapon => "WPN",
        EquipmentSlot::Shield => "SHLD",
        EquipmentSlot::Helmet => "HELM",
        EquipmentSlot::Body => "BODY",
        EquipmentSlot::Accessory => "ACC",
    }
}
