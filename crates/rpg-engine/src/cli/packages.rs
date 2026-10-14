//! Scenario package selection beneath the bounded collection, and the asset base it lives in.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    scenario_cross_reference::{
        DiagnosticSeverity, ScenarioDiagnostic, ScenarioLocation, ScenarioValidationReport,
    },
    scenario_path::ScenarioRelativePath,
    scenario_root::{SCENARIO_MANIFEST_PATH, ScenarioRoot},
};

/// Why a package key could not be resolved to a usable scenario package directory.
pub(super) struct PackageSelectionError {
    pub(super) code: &'static str,
    pub(super) message: &'static str,
}

/// Resolves `root`'s package key to a scenario package directory beneath `collection_root`,
/// rejecting anything that escapes the collection (a symlinked or `..`-traversing package key)
/// before any command reads from it. Shared by every scenario command so `validate-scenario` and
/// `map-report` reject the same package layouts the same way.
pub(super) fn select_scenario_package(
    collection_root: &Path,
    root: &ScenarioRoot,
) -> Result<PathBuf, PackageSelectionError> {
    let Ok(canonical_collection) = collection_root.canonicalize() else {
        return Err(PackageSelectionError {
            code: "scenario.collection_unavailable",
            message: "scenario collection is unavailable",
        });
    };
    if !canonical_collection.is_dir() {
        return Err(PackageSelectionError {
            code: "scenario.collection_unavailable",
            message: "scenario collection is not a directory",
        });
    }

    let selected = collection_root.join(root.package_key());
    let Ok(metadata) = fs::symlink_metadata(&selected) else {
        return Err(PackageSelectionError {
            code: "scenario.package_unavailable",
            message: "selected scenario package is unavailable",
        });
    };
    if metadata.file_type().is_symlink() {
        return Err(PackageSelectionError {
            code: "scenario.package_escape",
            message: "selected scenario package must not be a symbolic link",
        });
    }
    let Ok(canonical_selected) = selected.canonicalize() else {
        return Err(PackageSelectionError {
            code: "scenario.package_unavailable",
            message: "selected scenario package cannot be resolved",
        });
    };
    if !canonical_selected.starts_with(&canonical_collection) {
        return Err(PackageSelectionError {
            code: "scenario.package_escape",
            message: "selected scenario package resolves outside the scenario collection",
        });
    }
    if !canonical_selected.is_dir() {
        return Err(PackageSelectionError {
            code: "scenario.package_unavailable",
            message: "selected scenario package is not a directory",
        });
    }

    Ok(canonical_selected)
}

pub(super) fn selection_failure(
    root: &ScenarioRoot,
    code: &'static str,
    message: &'static str,
) -> ScenarioValidationReport {
    ScenarioValidationReport {
        package_key: root.package_key().to_owned(),
        diagnostics: vec![ScenarioDiagnostic {
            severity: DiagnosticSeverity::Error,
            code,
            location: ScenarioLocation {
                path: ScenarioRelativePath::try_from(SCENARIO_MANIFEST_PATH)
                    .expect("the manifest path is valid"),
                field_path: "$selection".to_owned(),
            },
            message: message.to_owned(),
        }],
        ..Default::default()
    }
}

pub(crate) fn production_asset_base() -> PathBuf {
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    match std::env::current_exe() {
        Ok(executable) => asset_base_for_executable(&executable, &manifest_directory),
        Err(_) => manifest_directory.join("assets"),
    }
}

pub(super) fn production_scenario_collection() -> PathBuf {
    production_asset_base().join("scenarios")
}

/// Resolves the Bevy asset base through the same development/installed layout decision as the
/// filesystem validator's scenario collection.
///
/// The repository target directory may resolve outside the repository when it is a symlink. Its
/// canonical location still identifies that executable as a development build without consulting
/// the process working directory. A packaged executable copied elsewhere continues to use only
/// the `assets/` directory beside itself.
fn asset_base_for_executable(executable: &Path, manifest_directory: &Path) -> PathBuf {
    let development_target_directory = manifest_directory.join("target");
    let is_development_executable = path_is_within(executable, manifest_directory)
        || path_is_within(executable, &development_target_directory);

    if is_development_executable {
        manifest_directory.join("assets")
    } else {
        executable
            .parent()
            .unwrap_or(manifest_directory)
            .join("assets")
    }
}

fn path_is_within(path: &Path, directory: &Path) -> bool {
    path.starts_with(directory)
        || fs::canonicalize(path)
            .ok()
            .zip(fs::canonicalize(directory).ok())
            .is_some_and(|(path, directory)| path.starts_with(directory))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::cli::tests::TempCollection;

    #[test]
    fn development_executable_uses_the_manifest_repository_collection() {
        let workspace = std::env::temp_dir().join("invented-workspace/rpg-s1");
        let executable = workspace.join("target/debug/rpg-s1");

        assert_eq!(
            asset_base_for_executable(&executable, &workspace).join("scenarios"),
            workspace.join("assets/scenarios")
        );
        assert_eq!(
            asset_base_for_executable(&executable, &workspace),
            workspace.join("assets")
        );
    }

    #[cfg(unix)]
    #[test]
    fn development_executable_behind_target_symlink_uses_manifest_assets() {
        use std::os::unix::fs::symlink;

        let temporary = TempCollection::new("source/rpg-s1");
        let manifest_directory = temporary.0.join("source/rpg-s1");
        let external_target = temporary.0.join("cache/target");
        let external_executable = external_target.join("debug/rpg-s1");
        fs::create_dir_all(external_executable.parent().unwrap())
            .expect("external target should be creatable");
        fs::write(&external_executable, b"invented executable")
            .expect("external executable should be creatable");
        symlink(&external_target, manifest_directory.join("target"))
            .expect("repository target symlink should be creatable");
        let resolved_executable = fs::canonicalize(manifest_directory.join("target/debug/rpg-s1"))
            .expect("target executable should resolve through the symlink");

        assert_eq!(
            asset_base_for_executable(&resolved_executable, &manifest_directory),
            manifest_directory.join("assets")
        );
    }

    #[test]
    fn installed_executable_uses_only_assets_beside_its_package_binary() {
        let base = std::env::temp_dir().join("invented-installed-layout");
        let manifest_directory = base.join("source/rpg-s1");
        let executable = base.join("package/rpg-s1");

        assert_eq!(
            asset_base_for_executable(&executable, &manifest_directory).join("scenarios"),
            base.join("package/assets/scenarios")
        );
        assert_eq!(
            asset_base_for_executable(&executable, &manifest_directory),
            base.join("package/assets")
        );
    }

    #[test]
    fn collection_resolution_is_absolute_and_has_no_working_directory_input() {
        let base = std::env::temp_dir().join("invented-cwd-independent-layout");
        let manifest_directory = base.join("source/rpg-s1");
        let executable = base.join("package/rpg-s1");

        let collection =
            asset_base_for_executable(&executable, &manifest_directory).join("scenarios");

        assert!(collection.is_absolute());
        assert_eq!(
            collection,
            executable.parent().unwrap().join("assets/scenarios")
        );
    }

    #[test]
    fn missing_installed_collection_does_not_trigger_an_ancestor_search() {
        let temporary = TempCollection::new("unrelated");
        let manifest_directory = temporary.0.join("source/rpg-s1");
        let executable = temporary.0.join("package/bin/rpg-s1");
        let unrelated_ancestor_collection = temporary.0.join("package/assets/scenarios");
        fs::create_dir_all(&unrelated_ancestor_collection)
            .expect("unrelated ancestor collection should be creatable");
        let intended = temporary.0.join("package/bin/assets/scenarios");
        assert!(!intended.exists());

        assert_eq!(
            asset_base_for_executable(&executable, &manifest_directory).join("scenarios"),
            intended
        );
    }
}
