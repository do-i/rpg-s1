use super::*;
use crate::field_menu::settings::{MUSIC_ROW, SFX_ROW, SOUND_ROW, sub_count};

#[expect(
    clippy::too_many_arguments,
    reason = "the settings page coordinates the shared menu root, catalog readiness, and live options"
)]
pub(in crate::field_menu) fn sync_settings_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    options: Option<Res<PlayerOptions>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuSettingsPage>>,
) {
    let show = state.open
        && state.screen == FieldMenuScreen::Settings
        && catalog.status() == CatalogStatus::Ready;
    if !show {
        for entity in &pages {
            commands.entity(entity).despawn();
        }
        return;
    }
    let Ok(menu_root) = menu_roots.single() else {
        return;
    };
    let options_changed = options.as_ref().is_some_and(|options| options.is_changed());
    if !(pages.is_empty() || state.is_changed() || catalog.is_changed() || options_changed) {
        return;
    }
    for entity in &pages {
        commands.entity(entity).despawn();
    }
    let Some(font) = scenario_font(&asset_server, &root, &inventory) else {
        return;
    };
    commands.entity(menu_root).with_children(|parent| {
        let default = PlayerOptions::default();
        let options = options.as_deref().unwrap_or(&default);
        spawn_settings_page(parent, &font, &state, options);
    });
}

pub(in crate::field_menu) fn spawn_settings_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    options: &PlayerOptions,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            FieldMenuSettingsPage,
            Name::new("Settings page"),
        ))
        .with_children(|page| {
            spawn_settings_header(page, font);
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Start,
                column_gap: px(STATUS_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                let in_sub = state.settings_sub.is_some();
                spawn_status_panel(columns, settings_panel_node(), "SETTINGS", font, |panel| {
                    spawn_settings_row(
                        panel,
                        font,
                        "Sound",
                        "Master switch for all audio",
                        if options.sound_enabled { "ON" } else { "OFF" },
                        options.sound_enabled,
                        state.selected == SOUND_ROW,
                        sub_count(SOUND_ROW) > 0,
                    );
                });
                if sub_count(state.selected) == 0 {
                    return;
                }
                spawn_status_panel(columns, settings_panel_node(), "SOUND", font, |panel| {
                    spawn_settings_row(
                        panel,
                        font,
                        "Music",
                        "Background music",
                        if options.music_enabled { "ON" } else { "OFF" },
                        options.sound_enabled && options.music_enabled,
                        in_sub && state.settings_sub == Some(MUSIC_ROW),
                        false,
                    );
                    spawn_settings_row(
                        panel,
                        font,
                        "Sound effects",
                        "Menu, battle and field effects",
                        if options.sfx_enabled { "ON" } else { "OFF" },
                        options.sound_enabled && options.sfx_enabled,
                        in_sub && state.settings_sub == Some(SFX_ROW),
                        false,
                    );
                });
            });
            spawn_status_text(
                page,
                "UP/DOWN   SELECT      ENTER / SPACE   TOGGLE      LEFT/RIGHT   LIST / OPTIONS      ESC   BACK",
                font,
                15.0,
                status_muted(),
            );
        });
}

/// Fixed width of each settings column. Wide enough for the label, subtitle, and value chip.
const SETTINGS_PANEL_WIDTH: f32 = 440.0;

fn settings_panel_node() -> Node {
    Node {
        width: px(SETTINGS_PANEL_WIDTH),
        flex_direction: FlexDirection::Column,
        row_gap: px(8),
        padding: UiRect::all(px(16)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    }
}

fn spawn_settings_header(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    parent
        .spawn(Node {
            width: percent(100),
            height: px(64),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            spawn_header_bars(header, 46.0, 15.0);
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "SETTINGS", font, 31.0, status_ink());
                    spawn_status_text(title, "PLAYER PREFERENCES", font, 14.0, status_muted());
                });
        });
}

#[expect(
    clippy::too_many_arguments,
    reason = "a row is its label, subtitle, value chip, two highlight states, and whether it opens a column"
)]
fn spawn_settings_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    name: &str,
    subtitle: &str,
    value: &str,
    active: bool,
    selected: bool,
    opens_column: bool,
) {
    let mut row = parent.spawn((
        Node {
            width: percent(100),
            height: px(46),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            padding: UiRect::axes(px(10), px(3)),
            border: UiRect::all(px(if selected { 2 } else { 1 })),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        selectable_card_colors(selected),
        SettingsRow,
        Name::new(format!("{name} setting")),
    ));
    if selected {
        row.insert(SelectedSettingsRow);
    }
    row.with_children(|row| {
        row.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|copy| {
            spawn_status_text(copy, name, font, 16.0, status_ink());
            spawn_status_text(copy, subtitle, font, 11.0, status_faint());
        });
        let color = if active { status_teal() } else { status_gold() };
        row.spawn((
            Node {
                min_width: px(90),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(10), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 170)),
            BorderColor::all(color),
        ))
        .with_children(|chip| {
            spawn_status_text(chip, value, font, 13.0, color);
        });
        if opens_column {
            // ASCII on purpose: the menu font has no arrow glyphs to rely on.
            spawn_status_text(row, ">", font, 18.0, status_gold());
        }
    });
}
