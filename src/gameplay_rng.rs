use bevy::prelude::*;

/// Deterministic seed used when startup does not supply one explicitly.
pub const DEFAULT_GAMEPLAY_SEED: u64 = 1;

/// The single deterministic random stream used by gameplay systems.
///
/// This resource implements SplitMix64 directly so its sequence is independent of dependency
/// upgrades and platform entropy. Its exact output is part of the save/replay compatibility
/// contract and is pinned by a golden-vector test below. It is suitable for gameplay simulation,
/// not cryptography.
#[derive(Debug, Resource)]
pub struct GameplayRng {
    state: u64,
}

impl Default for GameplayRng {
    fn default() -> Self {
        Self::from_seed(DEFAULT_GAMEPLAY_SEED)
    }
}

impl GameplayRng {
    /// Starts a deterministic stream at `seed`.
    pub const fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Restarts this stream at `seed`.
    pub fn reseed(&mut self, seed: u64) {
        self.state = seed;
    }

    /// Returns the next value from the fixed SplitMix64 stream.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}

/// Installs the deterministic gameplay stream with its startup default seed.
pub struct GameplayRngPlugin;

impl Plugin for GameplayRngPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameplayRng>();
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_GAMEPLAY_SEED, GameplayRng};

    fn sequence(seed: u64, length: usize) -> Vec<u64> {
        let mut rng = GameplayRng::from_seed(seed);
        (0..length).map(|_| rng.next_u64()).collect()
    }

    #[test]
    fn same_seed_repeats_sequence_across_independent_resources() {
        assert_eq!(sequence(42, 16), sequence(42, 16));
    }

    #[test]
    fn different_seeds_diverge() {
        assert_ne!(sequence(7, 8), sequence(8, 8));
    }

    #[test]
    fn reseed_restarts_the_selected_sequence() {
        let expected = sequence(99, 6);
        let mut rng = GameplayRng::from_seed(5);
        let _ = rng.next_u64();
        let _ = rng.next_u64();

        rng.reseed(99);

        assert_eq!(
            (0..expected.len())
                .map(|_| rng.next_u64())
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn splitmix64_output_contract_is_pinned() {
        assert_eq!(
            sequence(0, 5),
            [
                0xE220_A839_7B1D_CDAF,
                0x6E78_9E6A_A1B9_65F4,
                0x06C4_5D18_8009_454F,
                0xF88B_B8A8_724C_81EC,
                0x1B39_896A_51A8_749B,
            ]
        );
    }

    #[test]
    fn default_stream_uses_the_documented_seed() {
        let mut default_rng = GameplayRng::default();
        let mut explicitly_seeded = GameplayRng::from_seed(DEFAULT_GAMEPLAY_SEED);

        assert_eq!(default_rng.next_u64(), explicitly_seeded.next_u64());
    }
}
