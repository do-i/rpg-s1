use bevy::{
    camera::{ScalingMode, Viewport},
    prelude::*,
    ui::UiScale,
    window::PrimaryWindow,
};

pub(crate) mod camera_follow;

pub const LOGICAL_CANVAS_WIDTH: u32 = 1280;
pub const LOGICAL_CANVAS_HEIGHT: u32 = 766;

/// Applies the fixed gameplay canvas policy to every [`GameplayCanvasCamera`].
pub struct FixedGameplayCanvasPlugin;

impl Plugin for FixedGameplayCanvasPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiScale>()
            .add_systems(Update, update_gameplay_canvas);
    }
}

/// Marks a camera that renders the fixed-size gameplay canvas.
#[derive(Component)]
pub struct GameplayCanvasCamera;

/// Returns a 2D camera configured for the fixed logical gameplay area.
pub fn fixed_gameplay_camera() -> impl Bundle {
    (
        Camera2d,
        Msaa::Off,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: LOGICAL_CANVAS_WIDTH as f32,
                height: LOGICAL_CANVAS_HEIGHT as f32,
            },
            ..OrthographicProjection::default_2d()
        }),
        GameplayCanvasCamera,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PhysicalCanvasViewport {
    position: UVec2,
    size: UVec2,
}

impl PhysicalCanvasViewport {
    fn fit(window_size: UVec2) -> Option<Self> {
        if window_size.x == 0 || window_size.y == 0 {
            return None;
        }

        let window_width = u64::from(window_size.x);
        let window_height = u64::from(window_size.y);
        let logical_width = u64::from(LOGICAL_CANVAS_WIDTH);
        let logical_height = u64::from(LOGICAL_CANVAS_HEIGHT);

        let size = if window_width * logical_height <= window_height * logical_width {
            UVec2::new(
                window_size.x,
                ((window_width * logical_height) / logical_width) as u32,
            )
        } else {
            UVec2::new(
                ((window_height * logical_width) / logical_height) as u32,
                window_size.y,
            )
        }
        .max(UVec2::ONE);

        Some(Self {
            position: (window_size - size) / 2,
            size,
        })
    }

    fn viewport(self) -> Viewport {
        Viewport {
            physical_position: self.position,
            physical_size: self.size,
            ..default()
        }
    }
}

fn update_gameplay_canvas(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera, With<GameplayCanvasCamera>>,
    mut ui_scale: ResMut<UiScale>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let Some(viewport) = PhysicalCanvasViewport::fit(window.physical_size()) else {
        for mut camera in &mut cameras {
            camera.is_active = false;
            camera.viewport = None;
        }
        return;
    };

    for mut camera in &mut cameras {
        camera.is_active = true;
        camera.viewport = Some(viewport.viewport());
    }

    let physical_scale = (window.physical_width() as f32 / LOGICAL_CANVAS_WIDTH as f32)
        .min(window.physical_height() as f32 / LOGICAL_CANVAS_HEIGHT as f32);
    let window_scale_factor = window.scale_factor();
    if window_scale_factor.is_finite() && window_scale_factor > 0.0 {
        ui_scale.0 = physical_scale / window_scale_factor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::window::WindowResolution;

    fn viewport(width: u32, height: u32) -> Option<PhysicalCanvasViewport> {
        PhysicalCanvasViewport::fit(UVec2::new(width, height))
    }

    #[test]
    fn baseline_window_uses_the_full_canvas() {
        assert_eq!(
            viewport(1280, 766),
            Some(PhysicalCanvasViewport {
                position: UVec2::ZERO,
                size: UVec2::new(1280, 766),
            })
        );
    }

    #[test]
    fn smaller_window_is_centered_with_horizontal_bars() {
        assert_eq!(
            viewport(900, 600),
            Some(PhysicalCanvasViewport {
                position: UVec2::new(0, 31),
                size: UVec2::new(900, 538),
            })
        );
    }

    #[test]
    fn wider_window_is_centered_with_vertical_bars() {
        assert_eq!(
            viewport(1600, 900),
            Some(PhysicalCanvasViewport {
                position: UVec2::new(48, 0),
                size: UVec2::new(1503, 900),
            })
        );
    }

    #[test]
    fn portrait_window_uses_its_full_width() {
        assert_eq!(
            viewport(600, 900),
            Some(PhysicalCanvasViewport {
                position: UVec2::new(0, 270),
                size: UVec2::new(600, 359),
            })
        );
    }

    #[test]
    fn odd_bar_remainder_stays_on_the_right_or_bottom() {
        assert_eq!(
            viewport(999, 600),
            Some(PhysicalCanvasViewport {
                position: UVec2::new(0, 1),
                size: UVec2::new(999, 597),
            })
        );
        assert_eq!(600 - 1 - 597, 2);
    }

    #[test]
    fn zero_sized_windows_do_not_create_a_viewport() {
        assert_eq!(viewport(0, 0), None);
        assert_eq!(viewport(1280, 0), None);
        assert_eq!(viewport(0, 766), None);
    }

    #[test]
    fn plugin_configures_projection_viewport_and_hidpi_ui_scale() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(FixedGameplayCanvasPlugin);
        app.world_mut().spawn((
            Window {
                resolution: WindowResolution::new(1800, 1200).with_scale_factor_override(2.0),
                ..default()
            },
            PrimaryWindow,
        ));
        app.world_mut().spawn(fixed_gameplay_camera());

        app.update();

        let world = app.world_mut();
        let (camera, projection, msaa) = world
            .query_filtered::<(&Camera, &Projection, &Msaa), With<GameplayCanvasCamera>>()
            .single(world)
            .expect("one gameplay canvas camera");
        let configured_viewport = camera.viewport.as_ref().expect("a nonzero viewport");
        assert!(camera.is_active);
        assert_eq!(configured_viewport.physical_position, UVec2::new(0, 61));
        assert_eq!(configured_viewport.physical_size, UVec2::new(1800, 1077));

        let Projection::Orthographic(projection) = projection else {
            panic!("gameplay canvas must use an orthographic projection");
        };
        let ScalingMode::Fixed { width, height } = projection.scaling_mode else {
            panic!("gameplay canvas must use a fixed logical projection");
        };
        assert_eq!(width, LOGICAL_CANVAS_WIDTH as f32);
        assert_eq!(height, LOGICAL_CANVAS_HEIGHT as f32);
        assert_eq!(*msaa, Msaa::Off);
        assert_eq!(world.resource::<UiScale>().0, 0.703125);
    }

    #[test]
    fn plugin_deactivates_a_camera_for_a_minimized_window() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(FixedGameplayCanvasPlugin);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: WindowResolution::new(0, 0),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.world_mut().spawn(fixed_gameplay_camera());

        app.update();

        {
            let camera = app
                .world_mut()
                .query_filtered::<&Camera, With<GameplayCanvasCamera>>()
                .single(app.world())
                .expect("one gameplay canvas camera");
            assert!(!camera.is_active);
            assert!(camera.viewport.is_none());
        }

        app.world_mut()
            .entity_mut(window)
            .get_mut::<Window>()
            .expect("primary window")
            .resolution
            .set_physical_resolution(LOGICAL_CANVAS_WIDTH, LOGICAL_CANVAS_HEIGHT);
        app.update();

        let camera = app
            .world_mut()
            .query_filtered::<&Camera, With<GameplayCanvasCamera>>()
            .single(app.world())
            .expect("one gameplay canvas camera");
        assert!(camera.is_active);
        assert_eq!(
            camera
                .viewport
                .as_ref()
                .map(|viewport| viewport.physical_size),
            Some(UVec2::new(LOGICAL_CANVAS_WIDTH, LOGICAL_CANVAS_HEIGHT))
        );
    }
}
