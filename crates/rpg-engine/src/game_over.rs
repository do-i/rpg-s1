//! Retry, native-load, and title routing after total party defeat.

use bevy::prelude::*;

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    encounter::BattleEntry,
    game_state::GameState,
    gameplay_canvas::fixed_gameplay_camera,
    save_ui::OpenTitleLoadPicker,
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    sfx_cue::{MenuSfx, PlaySfx},
    ui_theme::UiTheme,
};

const OPTIONS: [&str; 3] = ["Retry Battle", "Load Game", "Title Screen"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GameOverAction {
    Retry,
    Load,
    Title,
}

const fn selected_action(selected: usize) -> GameOverAction {
    match selected {
        0 => GameOverAction::Retry,
        1 => GameOverAction::Load,
        _ => GameOverAction::Title,
    }
}

pub(crate) struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .init_resource::<GameOverMenu>()
            .add_systems(OnEnter(AppState::GameOver), enter_game_over)
            .add_systems(
                Update,
                (handle_game_over_input, sync_game_over_ui)
                    .chain()
                    .run_if(in_state(AppState::GameOver)),
            )
            .add_systems(OnExit(AppState::GameOver), cleanup_game_over);
    }
}

#[derive(Debug, Default, Resource)]
struct GameOverMenu {
    selected: usize,
}

#[derive(Component)]
struct GameOverUi;

#[derive(Component)]
struct GameOverOptions;

fn enter_game_over(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    theme: Res<UiTheme>,
    mut menu: ResMut<GameOverMenu>,
) {
    *menu = GameOverMenu::default();
    commands.spawn((fixed_gameplay_camera(), GameOverUi));
    let Some(font_path) = inventory.font.as_ref() else {
        return;
    };
    let font = asset_server.load(root.resolve(font_path));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(32),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.01, 0.03, 0.98)),
            GlobalZIndex(6_000),
            GameOverUi,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("GAME OVER"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(64.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(190, 55, 55)),
            ));
            root.spawn((
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(theme.menu_font_size),
                    ..default()
                },
                TextColor(theme.name_entry_input_color),
                TextLayout::justify(Justify::Center),
                GameOverOptions,
            ));
        });
}

fn handle_game_over_input(
    mut commands: Commands,
    actions: Res<ActionState>,
    mut menu: ResMut<GameOverMenu>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
    mut menu_sfx: MenuSfx,
) {
    if let Some(movement) = actions.menu_navigation() {
        menu.selected =
            (menu.selected as isize + movement).rem_euclid(OPTIONS.len() as isize) as usize;
        menu_sfx.hover();
    }
    if !actions.just_pressed(AppAction::Confirm) {
        return;
    }
    menu_sfx.confirm();
    match selected_action(menu.selected) {
        GameOverAction::Retry => {
            transitions.write(AppStateTransitionRequest::new(AppState::Battle));
        }
        GameOverAction::Load => {
            commands.remove_resource::<BattleEntry>();
            commands.remove_resource::<GameState>();
            commands.insert_resource(OpenTitleLoadPicker);
            transitions.write(AppStateTransitionRequest::new(AppState::Title));
        }
        GameOverAction::Title => {
            commands.remove_resource::<BattleEntry>();
            commands.remove_resource::<GameState>();
            transitions.write(AppStateTransitionRequest::new(AppState::Title));
        }
    };
}

fn sync_game_over_ui(
    menu: Res<GameOverMenu>,
    mut options: Query<&mut Text, With<GameOverOptions>>,
) {
    let Ok(mut text) = options.single_mut() else {
        return;
    };
    text.0 = OPTIONS
        .iter()
        .enumerate()
        .map(|(index, label)| {
            format!(
                "{}{}",
                if index == menu.selected { "> " } else { "  " },
                label
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
}

fn cleanup_game_over(mut commands: Commands, entities: Query<Entity, With<GameOverUi>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_over_contract_exposes_retry_load_and_title_in_source_order() {
        assert_eq!(OPTIONS, ["Retry Battle", "Load Game", "Title Screen"]);
        assert_eq!(selected_action(0), GameOverAction::Retry);
        assert_eq!(selected_action(1), GameOverAction::Load);
        assert_eq!(selected_action(2), GameOverAction::Title);
    }
}
