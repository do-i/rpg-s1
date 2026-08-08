//! Typed schema slices for the scenario manifest.
//!
//! Each milestone adds only the source fields it owns. Serde's default unknown-field
//! behavior is intentional: a partial schema must be able to load the pinned complete
//! manifest while its other sections are introduced by later milestones.

use serde::Deserialize;

/// The manifest fields that identify scenario content and label its game window.
///
/// `id` and `version` are content identity rather than the selected package key; see
/// ADR 0004. The fields are source-authored strings so their exact values remain available
/// to save, cache, recording, and UI systems that are added later.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestIdentityWindow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub window_title: String,
}

#[cfg(test)]
mod tests {
    use super::ManifestIdentityWindow;
    use crate::scenario_yaml;

    #[test]
    fn loads_pinned_identity_and_window_values_from_complete_manifest_shape() {
        let manifest: ManifestIdentityWindow = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/rusted-kingdoms-manifest-identity-window.yaml"
        ))
        .expect("the pinned manifest identity/window slice should deserialize");

        assert_eq!(
            manifest,
            ManifestIdentityWindow {
                id: "my_rpg_story".to_owned(),
                name: "Chronicles of the Lost Flame".to_owned(),
                version: "1.0.0".to_owned(),
                window_title: "Rusted Kingdoms".to_owned(),
            }
        );
    }
}
