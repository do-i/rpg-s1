//! Fails if a scenario package path is spelled out in Rust source again.
//!
//! Every reference to the shipped package must go through the per-crate helpers in
//! `test_support.rs` (`scenario_file!`, `scenario_asset!`, `scenario_package_dir()`), so renaming
//! or swapping a scenario package is a one-line edit per crate instead of a repository-wide sweep.
//!
//! Runs on every `cargo test`.

use std::{fs, path::Path};

/// Built from fragments so this guard does not match its own source.
fn needle() -> String {
    format!("scenarios/{}_{}", "rusted", "kingdoms")
}

/// Files permitted to spell the package path out.
///
/// `test_support.rs` is where each crate defines it once. `scenario_root.rs` holds the tests that
/// pin the `scenarios/<key>/...` layout itself, which must keep literal expectations.
const ALLOWED: &[&str] = &[
    "crates/rpg-content/src/test_support.rs",
    "crates/rpg-content/src/scenario_root.rs",
    "crates/rpg-engine/src/test_support.rs",
];

/// Directories whose Rust sources are subject to the guard.
const SCANNED: &[&str] = &["crates", "src", "tests"];

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rust_sources(directory: &Path, found: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            // `target` holds build output, not authored source.
            if name != "target" && name != ".git" {
                collect_rust_sources(&path, found);
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn scenario_package_paths_stay_behind_the_test_support_helpers() {
    let root = workspace_root();
    let needle = needle();

    let mut sources = Vec::new();
    for directory in SCANNED {
        collect_rust_sources(&root.join(directory), &mut sources);
    }
    assert!(
        sources.len() > 50,
        "the guard scanned only {} files; its directory list is likely wrong",
        sources.len()
    );

    let mut offenders = Vec::new();
    for path in sources {
        let relative = path
            .strip_prefix(root)
            .expect("scanned paths start at the workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        if ALLOWED.contains(&relative.as_str()) {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in contents.lines().enumerate() {
            if line.contains(&needle) {
                offenders.push(format!("  {relative}:{}\n     {}", index + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "scenario package paths must not be spelled out in Rust source.\n\
         Use the helpers in the crate's `test_support.rs` instead:\n\
           scenario_file!(\"data/...\")      compile-time include_str!/include_bytes! path\n\
           scenario_asset!(\"media/...\")     AssetServer-logical path\n\
           scenario_package_dir()           filesystem path for runtime reads\n\n\
         Offending lines:\n{}",
        offenders.join("\n")
    );
}
