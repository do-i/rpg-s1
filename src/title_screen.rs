use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};

const MENU_LABELS: [&str; 3] = ["New Game", "Load Game", "Quit"];
const LOAD_GAME_INDEX: usize = 1;
const NORMAL_COLOR: Color = Color::srgb_u8(170, 140, 100);
const SELECTED_COLOR: Color = Color::srgb_u8(220, 140, 60);
const DISABLED_COLOR: Color = Color::srgb_u8(80, 70, 55);

pub struct TitleScreenPlugin;

impl Plugin for TitleScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TitleMenu>()
            .add_systems(Startup, setup_title_screen)
            .add_systems(Update, (handle_menu_input, update_menu_colors));
    }
}

#[derive(Resource, Default)]
struct TitleMenu {
    selected: usize,
}

impl TitleMenu {
    fn move_by(&mut self, delta: isize) {
        self.selected =
            (self.selected as isize + delta).rem_euclid(MENU_LABELS.len() as isize) as usize;
    }
}

#[derive(Component)]
struct MenuEntry(usize);

#[derive(Component)]
struct StatusMessage;

fn setup_title_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands.spawn(Sprite::from_image(
        asset_server.load("images/title_lost_flame.webp"),
    ));

    commands.spawn((
        AudioPlayer::new(asset_server.load("audio/title_theme.mp3")),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(0.65),
            ..default()
        },
    ));

    let font = asset_server.load("fonts/Philosopher-Regular.ttf");

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::End,
            padding: UiRect::bottom(px(32)),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(300),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(px(30), px(20)),
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.11)),
            ))
            .with_children(|panel| {
                for (index, label) in MENU_LABELS.into_iter().enumerate() {
                    panel.spawn((
                        Text::new(label),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(30.0),
                            ..default()
                        },
                        TextColor(if index == 0 {
                            SELECTED_COLOR
                        } else if index == LOAD_GAME_INDEX {
                            DISABLED_COLOR
                        } else {
                            NORMAL_COLOR
                        }),
                        TextLayout::justify(Justify::Center),
                        Node {
                            height: px(42),
                            ..default()
                        },
                        MenuEntry(index),
                    ));
                }
            });

            root.spawn((
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(220, 190, 145)),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(8),
                    left: px(0),
                    right: px(0),
                    ..default()
                },
                StatusMessage,
            ));
        });
}

fn handle_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<TitleMenu>,
    mut status: Single<&mut Text, With<StatusMessage>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let direction = if keys.just_pressed(KeyCode::ArrowUp) {
        Some(-1)
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        Some(1)
    } else {
        None
    };

    if let Some(delta) = direction {
        menu.move_by(delta);
        commands.spawn((
            AudioPlayer::new(asset_server.load("audio/menu_hover.mp3")),
            PlaybackSettings::DESPAWN,
        ));
        status.0.clear();
    }

    if !(keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space)) {
        return;
    }

    match menu.selected {
        0 => {
            commands.spawn((
                AudioPlayer::new(asset_server.load("audio/menu_confirm.mp3")),
                PlaybackSettings::DESPAWN,
            ));
            status.0 = "New Game is the next migration slice.".into();
        }
        LOAD_GAME_INDEX => {}
        2 => {
            commands.spawn((
                AudioPlayer::new(asset_server.load("audio/menu_confirm.mp3")),
                PlaybackSettings::DESPAWN,
            ));
            commands.write_message(AppExit::Success);
        }
        _ => unreachable!(),
    }
}

fn update_menu_colors(menu: Res<TitleMenu>, mut entries: Query<(&MenuEntry, &mut TextColor)>) {
    if !menu.is_changed() {
        return;
    }

    for (entry, mut color) in &mut entries {
        color.0 = if entry.0 == LOAD_GAME_INDEX {
            DISABLED_COLOR
        } else if entry.0 == menu.selected {
            SELECTED_COLOR
        } else {
            NORMAL_COLOR
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_navigation_wraps_in_both_directions() {
        let mut menu = TitleMenu::default();

        menu.move_by(-1);
        assert_eq!(menu.selected, 2);

        menu.move_by(1);
        assert_eq!(menu.selected, 0);
    }
}
