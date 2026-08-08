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
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            clear_color: Color::srgb_u8(10, 10, 30),
            panel_color: Color::srgba(0.0, 0.0, 0.0, 0.11),
            menu_normal_color: Color::srgb_u8(170, 140, 100),
            menu_selected_color: Color::srgb_u8(220, 140, 60),
            menu_disabled_color: Color::srgb_u8(80, 70, 55),
            status_color: Color::srgb_u8(220, 190, 145),
            menu_font_size: 30.0,
            status_font_size: 17.0,
        }
    }
}
