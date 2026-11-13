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
    /// The multiplier `with_font_scale` applied, kept so literal sizes can follow it via [`Self::scaled`].
    pub(crate) font_scale: f32,
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
            font_scale: 1.0,
        }
    }
}

impl UiTheme {
    /// Full-screen menu plate behind the save and options screens.
    pub(crate) const OVERLAY_PANEL: Color = Color::srgba(0.02, 0.02, 0.08, 0.96);
    /// The service (shop) panel; a hair off [`Self::OVERLAY_PANEL`] in every channel, kept as shipped.
    pub(crate) const SERVICE_PANEL: Color = Color::srgba(0.025, 0.025, 0.09, 0.97);
    /// Half-opaque black dim laid over the world while the game is paused.
    pub(crate) const DIM_BACKDROP: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
    /// Heavier black dim for scene transitions over the world.
    pub(crate) const SCENE_DIM_BACKDROP: Color = Color::srgba(0.0, 0.0, 0.0, 0.85);
    /// Near-black backdrop of the game-over screen.
    pub(crate) const GAME_OVER_BACKDROP: Color = Color::srgba(0.015, 0.01, 0.03, 0.98);
    /// Opaque backdrop of the credits roll.
    pub(crate) const CREDITS_BACKDROP: Color = Color::srgba(0.01, 0.008, 0.02, 1.0);
    /// The dialogue box plate.
    pub(crate) const DIALOGUE_PANEL: Color = Color::srgba(0.047, 0.047, 0.118, 0.94);

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
            font_scale: scale,
            ..base
        }
    }

    /// An arbitrary text size multiplied by the same scale the named sizes use.
    ///
    /// For screens whose text is not one of the named roles; at the shipped scale of 1.0 the size
    /// comes back unchanged.
    pub(crate) fn scaled(&self, size: f32) -> f32 {
        size * self.font_scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_and_backdrop_tokens_keep_their_shipped_components() {
        assert_eq!(UiTheme::OVERLAY_PANEL, Color::srgba(0.02, 0.02, 0.08, 0.96));
        assert_eq!(
            UiTheme::SERVICE_PANEL,
            Color::srgba(0.025, 0.025, 0.09, 0.97)
        );
        assert_eq!(UiTheme::DIM_BACKDROP, Color::srgba(0.0, 0.0, 0.0, 0.55));
        assert_eq!(
            UiTheme::SCENE_DIM_BACKDROP,
            Color::srgba(0.0, 0.0, 0.0, 0.85)
        );
        assert_eq!(
            UiTheme::GAME_OVER_BACKDROP,
            Color::srgba(0.015, 0.01, 0.03, 0.98)
        );
        assert_eq!(
            UiTheme::CREDITS_BACKDROP,
            Color::srgba(0.01, 0.008, 0.02, 1.0)
        );
        assert_eq!(
            UiTheme::DIALOGUE_PANEL,
            Color::srgba(0.047, 0.047, 0.118, 0.94)
        );
    }

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
    fn scaled_applies_the_same_factor_to_any_size() {
        assert_eq!(UiTheme::default().scaled(64.0), 64.0);
        assert_eq!(UiTheme::with_font_scale(1.5).scaled(64.0), 96.0);
        assert_eq!(UiTheme::with_font_scale(0.0).scaled(64.0), 64.0);
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
