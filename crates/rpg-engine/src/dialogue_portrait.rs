//! Shared shoulder-up portrait framing for NPC-backed popup dialogue.
//!
//! World dialogue and service overlays both source their speaker art from the idle Down frame of
//! an NPC walking sheet. Keeping the frame selection and crop here prevents a new popup from
//! accidentally stretching the character's entire body into a portrait slot.

use bevy::prelude::*;

use crate::tsx_atlas_asset::TsxAtlasAsset;

/// The idle Down frame used by the source for dialogue portraits: row 2, frame 0.
pub(crate) const DIALOGUE_PORTRAIT_SOURCE_FRAME: usize = 18;

/// `PORTRAIT_HEAD_TOP_RATIO` in the pinned source's `engine/world/sprite_sheet.py`.
const DIALOGUE_PORTRAIT_TOP_RATIO: f32 = 10.0 / 64.0;

/// One shoulder-up portrait selected from a larger sprite sheet.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DialoguePortrait {
    pub(crate) image: Handle<Image>,
    /// Absolute pixel region of the source sheet, cropped to the speaker's head and shoulders.
    pub(crate) rect: Rect,
}

impl DialoguePortrait {
    /// Produces a UI image that displays only the shared shoulder-up crop.
    pub(crate) fn image_node(&self) -> ImageNode {
        let mut image = ImageNode::new(self.image.clone());
        image.rect = Some(self.rect);
        image
    }
}

/// Crops a speaker's head and shoulders out of one walking-sheet frame.
///
/// This reproduces the source's `get_portrait` integer truncation: the middle half of the frame
/// horizontally and the upper half beginning slightly below the frame edge.
pub(crate) fn dialogue_portrait_crop(frame: Rect) -> Rect {
    let width = frame.width();
    let height = frame.height();
    let crop = Vec2::new((width * 0.5).trunc(), (height * 0.5).trunc());
    let origin = frame.min
        + Vec2::new(
            (width / 4.0).trunc(),
            (height * DIALOGUE_PORTRAIT_TOP_RATIO).trunc(),
        );
    Rect::from_corners(origin, origin + crop)
}

/// Resolves a dialogue portrait from the sprite already standing in the world.
pub(crate) fn dialogue_portrait_from_sprite(
    sprite: &Sprite,
    layouts: &Assets<TextureAtlasLayout>,
) -> Option<DialoguePortrait> {
    let layout = layouts.get(&sprite.texture_atlas.as_ref()?.layout)?;
    let frame = layout.textures.get(DIALOGUE_PORTRAIT_SOURCE_FRAME)?;
    Some(DialoguePortrait {
        image: sprite.image.clone(),
        rect: dialogue_portrait_crop(frame.as_rect()),
    })
}

/// Resolves the same dialogue portrait from a manifest-declared TSX atlas.
pub(crate) fn dialogue_portrait_from_atlas(atlas: &TsxAtlasAsset) -> Option<DialoguePortrait> {
    let frame = atlas.frame_rect(DIALOGUE_PORTRAIT_SOURCE_FRAME as u32)?;
    Some(DialoguePortrait {
        image: atlas.image().clone(),
        rect: dialogue_portrait_crop(frame),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_popup_uses_the_same_shoulder_up_crop() {
        let frame = Rect::new(0.0, 128.0, 64.0, 192.0);

        let portrait = dialogue_portrait_crop(frame);

        assert_eq!(portrait.min, Vec2::new(16.0, 138.0));
        assert_eq!(portrait.width(), 32.0);
        assert_eq!(portrait.height(), 32.0);
        assert!(portrait.max.x <= frame.max.x && portrait.max.y <= frame.max.y);
    }

    #[test]
    fn crop_math_truncates_like_the_source() {
        let portrait = dialogue_portrait_crop(Rect::new(0.0, 0.0, 33.0, 33.0));

        assert_eq!(portrait.width(), 16.0);
        assert_eq!(portrait.height(), 16.0);
        assert_eq!(portrait.min, Vec2::new(8.0, 5.0));
    }

    #[test]
    fn image_nodes_retain_the_crop_instead_of_showing_the_full_sheet() {
        let portrait = DialoguePortrait {
            image: Handle::default(),
            rect: Rect::new(16.0, 10.0, 48.0, 42.0),
        };

        assert_eq!(portrait.image_node().rect, Some(portrait.rect));
    }
}
