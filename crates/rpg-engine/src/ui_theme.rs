use bevy::prelude::*;

/// Shared visual contract for the game's user interface.
///
/// Keeping these values in one resource lets later screens share the same palette and type scale
/// without copying title-screen constants.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct UiTheme {
    pub(crate) clear_color: Color,
    pub(crate) panel_color: Color,
    pub(crate) menu_normal_color: Color,
    pub(crate) menu_selected_color: Color,
    pub(crate) menu_disabled_color: Color,
    pub(crate) status_color: Color,
    pub(crate) menu_font_size: f32,
    pub(crate) status_font_size: f32,
    pub(crate) name_entry_prompt_color: Color,
    pub(crate) name_entry_box_color: Color,
    pub(crate) name_entry_border_color: Color,
    pub(crate) name_entry_input_color: Color,
    pub(crate) name_entry_hint_color: Color,
    pub(crate) name_entry_prompt_font_size: f32,
    pub(crate) name_entry_input_font_size: f32,
    pub(crate) name_entry_hint_font_size: f32,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            clear_color: Color::srgb_u8(10, 10, 30),
            // The source *intends* a half-opaque plate -- `title_scene.py` says "with a
            // 50%-opacity box" directly above `pygame.draw.rect(box_surf, (0, 0, 0, 28), ...)`,
            // and 28/255 is 11%, not 50%. The port inherited the 11% and with it the source's
            // unreadable menu: the box lands on the bottom-centre of `title_lost_flame.webp`,
            // which is the pale warm glow at the path's vanishing point, so warm menu text had
            // almost nothing to sit against. This honours the comment over the constant.
            panel_color: Color::srgba(0.0, 0.0, 0.0, 0.5),
            // Weathered parchment, lit ember, cold ash -- the title art's own three notes. Read
            // against the plate above the pale glow these hold roughly 3.3:1, 4.4:1 and 1.7:1,
            // so selection now separates by brightness as well as hue while the disabled entry
            // stays legibly dimmed rather than vanishing.
            menu_normal_color: Color::srgb_u8(195, 186, 168),
            menu_selected_color: Color::srgb_u8(255, 209, 102),
            menu_disabled_color: Color::srgb_u8(138, 129, 117),
            status_color: Color::srgb_u8(220, 190, 145),
            menu_font_size: 30.0,
            status_font_size: 17.0,
            name_entry_prompt_color: Color::srgb_u8(180, 180, 140),
            name_entry_box_color: Color::srgb_u8(40, 40, 70),
            name_entry_border_color: Color::srgb_u8(180, 180, 100),
            name_entry_input_color: Color::srgb_u8(255, 220, 80),
            name_entry_hint_color: Color::srgb_u8(120, 120, 100),
            name_entry_prompt_font_size: 36.0,
            name_entry_input_font_size: 48.0,
            name_entry_hint_font_size: 24.0,
        }
    }
}

impl UiTheme {
    /// The theme with every named font size multiplied by `scale`.
    ///
    /// This is where `fonts.sizes` from `assets/settings.yaml` lands. The source keeps a
    /// four-entry size palette; this engine names its type roles instead, so the setting is
    /// honored as a multiplier over those roles. A non-finite or non-positive scale is ignored
    /// rather than collapsing every label to nothing.
    pub(crate) fn with_font_scale(scale: f32) -> Self {
        let base = Self::default();
        if !scale.is_finite() || scale <= 0.0 || scale == 1.0 {
            return base;
        }
        Self {
            menu_font_size: base.menu_font_size * scale,
            status_font_size: base.status_font_size * scale,
            name_entry_prompt_font_size: base.name_entry_prompt_font_size * scale,
            name_entry_input_font_size: base.name_entry_input_font_size * scale,
            name_entry_hint_font_size: base.name_entry_hint_font_size * scale,
            ..base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_scale_leaves_every_size_untouched() {
        assert_eq!(UiTheme::with_font_scale(1.0), UiTheme::default());
    }

    #[test]
    fn a_scale_multiplies_every_named_size_and_no_colour() {
        let base = UiTheme::default();
        let scaled = UiTheme::with_font_scale(1.5);

        assert_eq!(scaled.menu_font_size, base.menu_font_size * 1.5);
        assert_eq!(scaled.status_font_size, base.status_font_size * 1.5);
        assert_eq!(
            scaled.name_entry_input_font_size,
            base.name_entry_input_font_size * 1.5
        );
        assert_eq!(
            scaled.menu_selected_color, base.menu_selected_color,
            "a type scale must not touch the palette"
        );
    }

    #[test]
    fn a_scale_that_would_erase_the_interface_is_refused() {
        for scale in [0.0, -2.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                UiTheme::with_font_scale(scale),
                UiTheme::default(),
                "scale {scale} must fall back to the shipped sizes"
            );
        }
    }
}
