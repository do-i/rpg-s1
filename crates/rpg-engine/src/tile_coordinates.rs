//! Shared conversion between top-left Tiled cells and Bevy world-space centers.

use bevy::math::Vec2;

/// Returns the Bevy world-space center of one finite orthogonal TMX cell.
///
/// TMX `(0, 0)` is the top-left cell. Columns increase along positive Bevy X,
/// rows increase along negative Bevy Y, and callers own the independent Z
/// coordinate used for render ordering.
pub(crate) fn tmx_tile_center(column: u32, row: u32, tile_width: u32, tile_height: u32) -> Vec2 {
    Vec2::new(
        (column as f32 + 0.5) * tile_width as f32,
        -(row as f32 + 0.5) * tile_height as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_top_left_and_bottom_right_ardel_cells_to_bevy_centers() {
        assert_eq!(tmx_tile_center(0, 0, 32, 32), Vec2::new(16.0, -16.0));
        assert_eq!(tmx_tile_center(29, 19, 32, 32), Vec2::new(944.0, -624.0));
    }
}
