//! The depth layout for v2 enemy battlers: three rows on the ground, nearer rows drawn larger,
//! lower, and on top.
//!
//! Approved against `temp/20261101-1103-v2-enemy-sizing/009_*` (five orcs on the mountain pass).
//! A battle uses it when any enemy has a v2 status sheet; a battle of v1 enemies only keeps the
//! legacy preset layout untouched.

/// One of the three ground rows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DepthRow {
    Back,
    Mid,
    Front,
}

impl DepthRow {
    /// The drawn size of a v2 battler tile in this row; 256 is the sheet's native tile.
    pub(super) const fn sprite_size(self) -> f32 {
        match self {
            Self::Back => 256.0,
            Self::Mid => 273.0,
            Self::Front => 300.0,
        }
    }

    /// The canvas y of the tile's bottom edge before the ground clamp.
    ///
    /// The approved mock put the painted feet on y = 372 / 405 / 455. The whole set sits 24px
    /// higher here so the front row's health bar still fits under its feet in the 468px area.
    pub(super) const fn tile_bottom(self) -> f32 {
        match self {
            Self::Back => 352.0,
            Self::Mid => 385.0,
            Self::Front => 436.0,
        }
    }

    /// Sibling draw order: a nearer row paints over a farther one.
    pub(super) const fn z(self) -> i32 {
        match self {
            Self::Back => 0,
            Self::Mid => 1,
            Self::Front => 2,
        }
    }
}

/// The canvas center x and row of one formation slot. Slots run left to right in formation order,
/// so target cycling still moves across the screen.
pub(super) fn depth_slot(count: usize, index: usize) -> (f32, DepthRow) {
    use DepthRow::{Back, Front, Mid};
    const ONE: [(f32, DepthRow); 1] = [(640.0, Front)];
    const TWO: [(f32, DepthRow); 2] = [(450.0, Front), (830.0, Front)];
    const THREE: [(f32, DepthRow); 3] = [(400.0, Front), (640.0, Front), (880.0, Front)];
    const FOUR: [(f32, DepthRow); 4] =
        [(330.0, Back), (450.0, Front), (830.0, Front), (950.0, Back)];
    const FIVE: [(f32, DepthRow); 5] = [
        (330.0, Back),
        (450.0, Front),
        (640.0, Mid),
        (830.0, Front),
        (950.0, Back),
    ];
    let slots = match count {
        0 | 1 => &ONE[..],
        2 => &TWO[..],
        3 => &THREE[..],
        4 => &FOUR[..],
        _ => &FIVE[..],
    };
    slots.get(index).copied().unwrap_or((640.0, Front))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_enemies_use_the_approved_rows_and_sizes() {
        let rows = (0..5)
            .map(|index| depth_slot(5, index))
            .map(|(x, row)| (x, row.sprite_size()))
            .collect::<Vec<_>>();
        assert_eq!(
            rows,
            vec![
                (330.0, 256.0),
                (450.0, 300.0),
                (640.0, 273.0),
                (830.0, 300.0),
                (950.0, 256.0),
            ]
        );
    }

    #[test]
    fn slots_run_left_to_right_and_nearer_rows_sit_lower_and_on_top() {
        for count in 1..=5 {
            let xs = (0..count)
                .map(|index| depth_slot(count, index).0)
                .collect::<Vec<_>>();
            assert!(
                xs.windows(2).all(|pair| pair[0] < pair[1]),
                "{count}: {xs:?}"
            );
        }
        let rows = [DepthRow::Back, DepthRow::Mid, DepthRow::Front];
        for pair in rows.windows(2) {
            assert!(pair[0].tile_bottom() < pair[1].tile_bottom());
            assert!(pair[0].z() < pair[1].z());
            assert!(pair[0].sprite_size() < pair[1].sprite_size());
        }
    }
}
