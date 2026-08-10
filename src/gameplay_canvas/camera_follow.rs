//! Bevy camera following within explicit map pixel bounds.
//!
//! The world camera uses the fixed gameplay-canvas dimensions rather than the physical window
//! or viewport. Each axis follows independently: a map larger than the logical canvas clamps at
//! its two edges, while a smaller map remains exactly centered on that axis.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.22 establishes the camera system before the later world plugin registers it"
    )
)]

use std::{error::Error, fmt};

use bevy::prelude::*;

use super::{GameplayCanvasCamera, LOGICAL_CANVAS_HEIGHT, LOGICAL_CANVAS_WIDTH};

const VIEW_WIDTH: f32 = LOGICAL_CANVAS_WIDTH as f32;
const VIEW_HEIGHT: f32 = LOGICAL_CANVAS_HEIGHT as f32;

/// Validated world-space pixel bounds for one finite map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MapPixelBounds {
    min: Vec2,
    max: Vec2,
}

impl MapPixelBounds {
    /// Creates bounds from the minimum world-space pixel corner and a positive pixel size.
    pub(crate) fn from_min_size(min: Vec2, size: Vec2) -> Result<Self, MapPixelBoundsError> {
        if !min.is_finite() {
            return Err(MapPixelBoundsError::MinimumNotFinite);
        }
        if !size.is_finite() {
            return Err(MapPixelBoundsError::SizeNotFinite);
        }
        if size.x <= 0.0 || size.y <= 0.0 {
            return Err(MapPixelBoundsError::SizeNotPositive);
        }
        let max = min + size;
        if !max.is_finite() {
            return Err(MapPixelBoundsError::MaximumNotFinite);
        }
        Ok(Self { min, max })
    }

    pub(crate) const fn min(self) -> Vec2 {
        self.min
    }

    pub(crate) const fn max(self) -> Vec2 {
        self.max
    }

    pub(crate) fn size(self) -> Vec2 {
        self.max - self.min
    }

    pub(crate) fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
}

/// Why finite map pixel bounds could not be constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MapPixelBoundsError {
    MinimumNotFinite,
    SizeNotFinite,
    SizeNotPositive,
    MaximumNotFinite,
}

impl fmt::Display for MapPixelBoundsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MinimumNotFinite => "map pixel minimum must be finite",
            Self::SizeNotFinite => "map pixel size must be finite",
            Self::SizeNotPositive => "map pixel width and height must be positive",
            Self::MaximumNotFinite => "map pixel maximum must be finite",
        })
    }
}

impl Error for MapPixelBoundsError {}

/// Marks the one logical player transform that the map camera follows.
#[derive(Component)]
pub(crate) struct CameraFollowTarget;

/// Attaches current finite-map bounds to a fixed gameplay-canvas camera.
#[derive(Clone, Copy, Component, Debug, PartialEq)]
pub(crate) struct MapCameraFollow {
    bounds: MapPixelBounds,
}

impl MapCameraFollow {
    pub(crate) const fn new(bounds: MapPixelBounds) -> Self {
        Self { bounds }
    }

    pub(crate) const fn bounds(self) -> MapPixelBounds {
        self.bounds
    }
}

type MapCameraFilter = (
    With<Camera2d>,
    With<GameplayCanvasCamera>,
    Without<CameraFollowTarget>,
);

/// Follows the logical player and clamps each gameplay camera to its current finite map.
pub(crate) fn follow_map_camera(
    targets: Query<&Transform, With<CameraFollowTarget>>,
    mut cameras: Query<(&MapCameraFollow, &mut Transform), MapCameraFilter>,
) {
    let Ok(target) = targets.single() else {
        return;
    };
    let target = target.translation.truncate();

    for (follow, mut camera) in &mut cameras {
        let bounds = follow.bounds();
        camera.translation.x = followed_axis(target.x, bounds.min().x, bounds.max().x, VIEW_WIDTH);
        camera.translation.y = followed_axis(target.y, bounds.min().y, bounds.max().y, VIEW_HEIGHT);
    }
}

fn followed_axis(target: f32, minimum: f32, maximum: f32, view_size: f32) -> f32 {
    let map_size = maximum - minimum;
    if map_size <= view_size {
        (minimum + maximum) * 0.5
    } else {
        let half_view = view_size * 0.5;
        target.clamp(minimum + half_view, maximum - half_view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LARGE_MAP_SIZE: Vec2 = Vec2::new(3200.0, 2400.0);
    const PLAYER_Z: f32 = 17.0;
    const CAMERA_Z: f32 = 999.0;

    fn bounds(size: Vec2) -> MapPixelBounds {
        MapPixelBounds::from_min_size(Vec2::ZERO, size).expect("test bounds should be valid")
    }

    fn run_follow(
        map_bounds: MapPixelBounds,
        player_position: Vec2,
        initial_camera_position: Vec2,
        updates: usize,
    ) -> (Vec3, Vec3) {
        let mut app = App::new();
        app.add_systems(Update, follow_map_camera);
        let player_transform = Transform {
            translation: player_position.extend(PLAYER_Z),
            rotation: Quat::from_rotation_z(0.25),
            scale: Vec3::new(2.0, 3.0, 1.0),
        };
        let player = app
            .world_mut()
            .spawn((player_transform, CameraFollowTarget))
            .id();
        let camera = app
            .world_mut()
            .spawn((
                Transform::from_xyz(
                    initial_camera_position.x,
                    initial_camera_position.y,
                    CAMERA_Z,
                ),
                Camera2d,
                GameplayCanvasCamera,
                MapCameraFollow::new(map_bounds),
            ))
            .id();

        let mut previous_camera_position = None;
        for _ in 0..updates {
            app.update();
            let current_camera_position = app
                .world()
                .entity(camera)
                .get::<Transform>()
                .expect("camera transform")
                .translation;
            if let Some(previous_camera_position) = previous_camera_position {
                assert_eq!(current_camera_position, previous_camera_position);
            }
            previous_camera_position = Some(current_camera_position);
        }

        let player_after = *app
            .world()
            .entity(player)
            .get::<Transform>()
            .expect("player transform");
        assert_eq!(player_after, player_transform);
        let camera_after = app
            .world()
            .entity(camera)
            .get::<Transform>()
            .expect("camera transform")
            .translation;
        (player_after.translation, camera_after)
    }

    #[test]
    fn large_map_centers_on_a_player_away_from_every_edge() {
        let (player, camera) = run_follow(
            bounds(LARGE_MAP_SIZE),
            Vec2::new(1700.0, 1200.0),
            Vec2::ZERO,
            1,
        );

        assert_eq!(player, Vec3::new(1700.0, 1200.0, PLAYER_Z));
        assert_eq!(camera, Vec3::new(1700.0, 1200.0, CAMERA_Z));
    }

    #[test]
    fn large_map_clamps_cleanly_at_each_of_its_four_edges() {
        for (player, expected_camera) in [
            (Vec2::new(0.0, 1200.0), Vec2::new(640.0, 1200.0)),
            (Vec2::new(3200.0, 1200.0), Vec2::new(2560.0, 1200.0)),
            (Vec2::new(1700.0, 0.0), Vec2::new(1700.0, 383.0)),
            (Vec2::new(1700.0, 2400.0), Vec2::new(1700.0, 2017.0)),
        ] {
            let (_, camera) = run_follow(bounds(LARGE_MAP_SIZE), player, Vec2::ZERO, 1);
            assert_eq!(camera.truncate(), expected_camera, "player at {player}");
            assert_eq!(camera.z, CAMERA_Z);
        }
    }

    #[test]
    fn map_smaller_than_the_canvas_in_both_dimensions_stays_centered() {
        let small = bounds(Vec2::new(800.0, 400.0));
        for player in [Vec2::ZERO, Vec2::new(400.0, 200.0), Vec2::new(800.0, 400.0)] {
            let (_, camera) = run_follow(small, player, Vec2::new(50.0, 75.0), 2);
            assert_eq!(camera, Vec3::new(400.0, 200.0, CAMERA_Z));
        }
    }

    #[test]
    fn only_a_small_width_centers_horizontally_while_vertical_following_remains_active() {
        let (_, camera) = run_follow(
            bounds(Vec2::new(800.0, 1800.0)),
            Vec2::new(25.0, 1100.0),
            Vec2::ZERO,
            2,
        );

        assert_eq!(camera, Vec3::new(400.0, 1100.0, CAMERA_Z));
    }

    #[test]
    fn only_a_small_height_centers_vertically_while_horizontal_following_remains_active() {
        let (_, camera) = run_follow(
            bounds(Vec2::new(2200.0, 300.0)),
            Vec2::new(1500.0, 275.0),
            Vec2::ZERO,
            2,
        );

        assert_eq!(camera, Vec3::new(1500.0, 150.0, CAMERA_Z));
    }

    #[test]
    fn explicit_nonzero_map_minimum_is_respected_without_changing_the_player() {
        let map_bounds =
            MapPixelBounds::from_min_size(Vec2::new(-400.0, -200.0), Vec2::new(2400.0, 1600.0))
                .unwrap();
        assert_eq!(map_bounds.size(), Vec2::new(2400.0, 1600.0));
        assert_eq!(map_bounds.center(), Vec2::new(800.0, 600.0));

        let (player, camera) = run_follow(
            map_bounds,
            Vec2::new(-400.0, -200.0),
            Vec2::new(1000.0, 1000.0),
            1,
        );
        assert_eq!(player, Vec3::new(-400.0, -200.0, PLAYER_Z));
        assert_eq!(camera, Vec3::new(240.0, 183.0, CAMERA_Z));
    }

    #[test]
    fn map_bounds_reject_nonfinite_and_nonpositive_inputs() {
        assert_eq!(
            MapPixelBounds::from_min_size(Vec2::new(f32::NAN, 0.0), Vec2::ONE),
            Err(MapPixelBoundsError::MinimumNotFinite)
        );
        assert_eq!(
            MapPixelBounds::from_min_size(Vec2::ZERO, Vec2::new(f32::INFINITY, 1.0)),
            Err(MapPixelBoundsError::SizeNotFinite)
        );
        assert_eq!(
            MapPixelBounds::from_min_size(Vec2::ZERO, Vec2::new(1.0, 0.0)),
            Err(MapPixelBoundsError::SizeNotPositive)
        );
        assert_eq!(
            MapPixelBounds::from_min_size(Vec2::splat(f32::MAX), Vec2::splat(f32::MAX)),
            Err(MapPixelBoundsError::MaximumNotFinite)
        );
    }
}
