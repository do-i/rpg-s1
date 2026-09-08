//! Mouse position, transformed into the fixed gameplay canvas.
//!
//! The window is not the canvas. [`super::update_gameplay_canvas`] fits a 1280x766 canvas into
//! whatever the window happens to be and centers it, so at almost every window size some of the
//! window is bar rather than game. A cursor position taken straight from the window is therefore
//! wrong twice over: it is offset by the bar, and it is scaled by however far the canvas had to
//! shrink. Both corrections happen here, once, so no screen has to remember to do them.
//!
//! Two spaces come out of that transform and both are needed:
//!
//! - **Canvas space** — `0..1280` by `0..766`, matching every hand-placed coordinate in the port.
//! - **Viewport space** — physical pixels measured from the viewport's top-left corner, which is
//!   the space Bevy's UI layout reports [`ComputedNode`] rects in, and so the space a hit test
//!   against a menu row has to use.
//!
//! A cursor in a letterbox or pillarbox bar is in neither space, and both are `None`. That is the
//! behavior the `DEFERRED-POINTER` acceptance asks for: bar space is not clamped to the nearest
//! edge, because clamping would let a click 200px outside the canvas activate the button nearest
//! the border.

use bevy::{
    input::mouse::MouseButton, prelude::*, ui::ComputedNode, ui::UiGlobalTransform,
    window::PrimaryWindow,
};

use super::{LOGICAL_CANVAS_HEIGHT, LOGICAL_CANVAS_WIDTH, PhysicalCanvasViewport};

/// A cursor position expressed in both spaces the port needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CanvasPointerPosition {
    /// Position inside the fixed logical canvas, `0..1280` by `0..766`.
    pub(crate) canvas: Vec2,
    /// Position in physical pixels from the viewport's top-left, for UI hit testing.
    pub(crate) viewport: Vec2,
}

/// Where the mouse is, and whether it was clicked this frame.
#[derive(Resource, Default)]
pub(crate) struct CanvasPointer {
    position: Option<CanvasPointerPosition>,
    clicked: bool,
}

impl CanvasPointer {
    /// Whether the primary button went down this frame while over the canvas.
    ///
    /// A click that began in a bar is not reported at all, rather than reported at the nearest
    /// canvas edge.
    pub(crate) fn just_clicked(&self) -> bool {
        self.clicked && self.position.is_some()
    }

    /// Whether the cursor is inside `node`, which must be a UI node laid out by the canvas camera.
    pub(crate) fn is_over(&self, node: &ComputedNode, transform: &UiGlobalTransform) -> bool {
        self.position.is_some_and(|position| {
            !node.is_empty() && node.contains_point(*transform, position.viewport)
        })
    }
}

/// Transforms a window cursor position into canvas and viewport space.
///
/// `cursor` is Bevy's logical window position, origin top-left. `window_physical` and
/// `scale_factor` come from the same [`Window`], and the scale factor is applied here rather than
/// by the caller because the viewport fit is computed in physical pixels.
pub(crate) fn map_cursor(
    window_physical: UVec2,
    scale_factor: f32,
    cursor: Vec2,
) -> Option<CanvasPointerPosition> {
    if !scale_factor.is_finite() || scale_factor <= 0.0 || !cursor.is_finite() {
        return None;
    }
    let viewport = PhysicalCanvasViewport::fit(window_physical)?;
    let physical = cursor * scale_factor;
    let relative = physical - viewport.position.as_vec2();
    let size = viewport.size.as_vec2();
    if relative.x < 0.0 || relative.y < 0.0 || relative.x >= size.x || relative.y >= size.y {
        return None;
    }

    // Scaled per axis rather than by one shared factor. The fit truncates to whole pixels, so the
    // two axes can differ by a fraction of a percent; using each axis's own ratio keeps the result
    // inside the canvas at the far edge instead of a hair past it.
    Some(CanvasPointerPosition {
        canvas: Vec2::new(
            relative.x * LOGICAL_CANVAS_WIDTH as f32 / size.x,
            relative.y * LOGICAL_CANVAS_HEIGHT as f32 / size.y,
        ),
        viewport: relative,
    })
}

pub(crate) struct CanvasPointerPlugin;

impl Plugin for CanvasPointerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CanvasPointer>()
            .add_systems(PreUpdate, update_canvas_pointer);
    }
}

fn update_canvas_pointer(
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut pointer: ResMut<CanvasPointer>,
) {
    pointer.clicked = buttons.is_some_and(|buttons| buttons.just_pressed(MouseButton::Left));
    pointer.position = windows.single().ok().and_then(|window| {
        let cursor = window.cursor_position()?;
        map_cursor(window.physical_size(), window.scale_factor(), cursor)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANVAS: Vec2 = Vec2::new(LOGICAL_CANVAS_WIDTH as f32, LOGICAL_CANVAS_HEIGHT as f32);

    fn canvas_of(window: (u32, u32), scale: f32, cursor: (f32, f32)) -> Option<Vec2> {
        map_cursor(
            UVec2::new(window.0, window.1),
            scale,
            Vec2::new(cursor.0, cursor.1),
        )
        .map(|position| position.canvas)
    }

    #[test]
    fn at_the_baseline_size_the_cursor_is_the_canvas_position() {
        assert_eq!(canvas_of((1280, 766), 1.0, (0.0, 0.0)), Some(Vec2::ZERO));
        assert_eq!(
            canvas_of((1280, 766), 1.0, (640.0, 383.0)),
            Some(Vec2::new(640.0, 383.0))
        );
    }

    #[test]
    fn a_pillarbox_bar_is_rejected_rather_than_clamped() {
        // fit(1600, 900) centers a 1503-wide viewport at x = 48.
        assert_eq!(canvas_of((1600, 900), 1.0, (47.0, 450.0)), None);
        assert_eq!(canvas_of((1600, 900), 1.0, (1551.0, 450.0)), None);
        // The first and last columns inside the bar do map.
        assert_eq!(canvas_of((1600, 900), 1.0, (48.0, 0.0)), Some(Vec2::ZERO));
        let last = canvas_of((1600, 900), 1.0, (1550.0, 450.0)).expect("inside the viewport");
        assert!(last.x < CANVAS.x, "the far edge stays inside the canvas");
    }

    #[test]
    fn a_letterbox_bar_is_rejected_rather_than_clamped() {
        // fit(900, 600) centers a 538-tall viewport at y = 31.
        assert_eq!(canvas_of((900, 600), 1.0, (450.0, 30.0)), None);
        assert_eq!(canvas_of((900, 600), 1.0, (450.0, 569.0)), None);
        assert_eq!(canvas_of((900, 600), 1.0, (0.0, 31.0)), Some(Vec2::ZERO));
    }

    #[test]
    fn the_viewport_center_is_the_canvas_center_at_every_window_size() {
        // The acceptance condition in one test: resizing must not move a point relative to the
        // canvas. Half a canvas pixel of tolerance covers the whole-pixel truncation in the fit.
        for window in [
            (1280, 766),
            (1600, 900),
            (900, 600),
            (600, 900),
            (3840, 2160),
            (321, 199),
        ] {
            let viewport =
                PhysicalCanvasViewport::fit(UVec2::new(window.0, window.1)).expect("a viewport");
            let center = viewport.position.as_vec2() + viewport.size.as_vec2() / 2.0;
            let mapped = canvas_of(window, 1.0, (center.x, center.y)).expect("the center maps");

            assert!(
                (mapped - CANVAS / 2.0).abs().max_element() <= 0.5,
                "{window:?} put the viewport center at {mapped:?}"
            );
        }
    }

    #[test]
    fn a_hidpi_scale_factor_is_applied_before_the_viewport_fit() {
        // A 1800x1200 physical window at 2x is 900x600 logical. The cursor arrives in logical
        // coordinates, so without the multiply it would land in the top-left quarter.
        let mapped = canvas_of((1800, 1200), 2.0, (450.0, 300.0)).expect("the center maps");

        assert!(
            (mapped - CANVAS / 2.0).abs().max_element() <= 0.5,
            "hidpi center landed at {mapped:?}"
        );
    }

    #[test]
    fn every_point_inside_the_viewport_maps_inside_the_canvas() {
        let window = UVec2::new(1600, 900);
        let viewport = PhysicalCanvasViewport::fit(window).expect("a viewport");

        for step in 0..=64 {
            let fraction = step as f32 / 64.0;
            let cursor = viewport.position.as_vec2()
                + (viewport.size.as_vec2() - Vec2::splat(1.0)) * fraction;
            let mapped = canvas_of((window.x, window.y), 1.0, (cursor.x, cursor.y))
                .expect("inside the viewport");

            assert!(
                mapped.x >= 0.0 && mapped.x < CANVAS.x && mapped.y >= 0.0 && mapped.y < CANVAS.y,
                "{cursor:?} mapped out of bounds to {mapped:?}"
            );
        }
    }

    #[test]
    fn a_degenerate_window_or_cursor_has_no_position() {
        assert_eq!(canvas_of((0, 0), 1.0, (10.0, 10.0)), None);
        assert_eq!(canvas_of((1280, 0), 1.0, (10.0, 10.0)), None);
        assert_eq!(canvas_of((1280, 766), 0.0, (10.0, 10.0)), None);
        assert_eq!(canvas_of((1280, 766), f32::NAN, (10.0, 10.0)), None);
        assert_eq!(canvas_of((1280, 766), 1.0, (f32::NAN, 10.0)), None);
        assert_eq!(canvas_of((1280, 766), 1.0, (-1.0, 10.0)), None);
    }

    #[test]
    fn viewport_space_is_measured_from_the_viewport_corner_not_the_window_corner() {
        let position = map_cursor(UVec2::new(1600, 900), 1.0, Vec2::new(148.0, 100.0))
            .expect("inside the viewport");

        // 148 physical px into the window is 100 into the viewport, because the bar is 48 wide.
        assert_eq!(position.viewport, Vec2::new(100.0, 100.0));
        // Canvas space additionally undoes the magnification. This window draws the 1280-wide
        // canvas across 1503 physical pixels, so 100 physical pixels is only ~85 canvas pixels —
        // the two spaces are genuinely different, not an offset apart.
        assert!(
            (position.canvas.x - 100.0 * 1280.0 / 1503.0).abs() < 0.01,
            "canvas x was {}",
            position.canvas.x
        );
        assert!(position.canvas.x < position.viewport.x);
    }

    #[test]
    fn a_click_in_a_bar_is_not_reported() {
        let mut pointer = CanvasPointer {
            position: None,
            clicked: true,
        };
        assert!(!pointer.just_clicked(), "a click needs a position");

        pointer.position = map_cursor(UVec2::new(1280, 766), 1.0, Vec2::new(4.0, 4.0));
        assert!(pointer.just_clicked());
    }
}
