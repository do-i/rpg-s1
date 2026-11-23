//! One soft elliptical ground shadow, shared by the field (NPCs, boss standees) and the battle
//! scene (enemy battlers). It is generated at runtime, so it adds no asset and no ledger row.
//!
//! Shadows ground a sprite on the 45-degree-camera floor and keep a figure readable against a
//! busy background. They are deliberately soft and weak: dark enough to anchor a foot, never
//! dark enough to read as a hole.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 32;
/// The outer share of the radius that fades to clear; inside it the core is uniformly dark. A
/// fade across the whole radius left too little dark area to read on grass or cobbles.
const EDGE_FADE: f32 = 0.45;
/// How dark a field shadow is. The texture's core is fully opaque, so each use sets its own
/// strength through the tint alpha.
pub(crate) const FIELD_OPACITY: f32 = 0.6;
/// How dark a battle shadow is: stronger than the field, against painted, dappled backgrounds.
pub(crate) const BATTLE_OPACITY: f32 = 0.8;
/// Child z under the owning sprite. Half the Y-sort step, so it never crosses a neighbour's band.
const CHILD_Z: f32 = -0.0005;

/// The shared shadow texture: an opaque black core falling off smoothly to a transparent rim.
pub(crate) fn shadow_image() -> Image {
    let mut data = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let dx = (x as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / HEIGHT as f32 * 2.0 - 1.0;
            let distance = (dx * dx + dy * dy).sqrt();
            let falloff = ((1.0 - distance) / EDGE_FADE).clamp(0.0, 1.0);
            // Smoothstep: a soft edge rather than a hard disc.
            let alpha = falloff * falloff * (3.0 - 2.0 * falloff);
            data.extend_from_slice(&[0, 0, 0, (alpha * 255.0).round() as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Returns the shared shadow handle, creating the texture on first use.
pub(crate) fn shared_handle(
    images: &mut Assets<Image>,
    cache: &mut Option<Handle<Image>>,
) -> Handle<Image> {
    cache
        .get_or_insert_with(|| images.add(shadow_image()))
        .clone()
}

/// A field shadow child, `size` pixels across, centred `feet_y` above (negative: below) its
/// parent's origin. The parent keeps ownership: despawning it takes the shadow along.
pub(crate) fn field_shadow(image: Handle<Image>, size: Vec2, feet_y: f32) -> (Sprite, Transform) {
    let mut sprite = Sprite::from_image(image);
    sprite.custom_size = Some(size);
    sprite.color = Color::srgba(1.0, 1.0, 1.0, FIELD_OPACITY);
    (sprite, Transform::from_xyz(0.0, feet_y, CHILD_Z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shadow_is_soft_centred_and_transparent_at_the_rim() {
        let image = shadow_image();
        let data = image.data.as_ref().expect("pixel data");
        let alpha = |x: u32, y: u32| data[((y * WIDTH + x) * 4 + 3) as usize];
        let centre = alpha(WIDTH / 2, HEIGHT / 2);
        assert_eq!(centre, 255, "the core is opaque; each use sets its opacity");
        // A solid core, so the shadow reads at its drawn size.
        assert_eq!(alpha(WIDTH / 4, HEIGHT / 2), centre);
        assert_eq!(alpha(0, 0), 0);
        assert!(alpha(WIDTH - 1, HEIGHT / 2) <= 2, "the rim is clear");
        let near_rim = alpha(WIDTH * 7 / 8, HEIGHT / 2);
        assert!(
            near_rim > 0 && near_rim < centre,
            "the rim fades: {near_rim}"
        );
    }
}
