//! Helpers shared by the windowless scenario report and sweep commands.
//!
//! Each report reads a scenario package from disk without failing: unreadable input yields an
//! empty report carrying a load error. The directory readers here refuse to leave the canonical
//! scenario root, and the macro gives every report type the same `with_load_error` constructor.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

/// Lists the files directly within `dir`, refusing to follow it outside `canonical_root`.
/// Every directory the report commands read (`data/maps`, `media/maps`, `data/dialogue`,
/// `data/items`) is flat in the shipped scenario, so this deliberately does not recurse.
pub(crate) fn safe_read_dir(dir: &Path, canonical_root: Option<&Path>) -> Option<Vec<PathBuf>> {
    let canonical_root = canonical_root?;
    let canonical_dir = dir.canonicalize().ok()?;
    if !canonical_dir.starts_with(canonical_root) || !canonical_dir.is_dir() {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    Some(
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect(),
    )
}

/// Collects the file stems of every file directly within `dir` that has `extension`.
pub(crate) fn read_stem_set(
    dir: &Path,
    canonical_root: Option<&Path>,
    extension: &str,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(paths) = safe_read_dir(dir, canonical_root) else {
        return out;
    };
    for path in paths {
        if path.extension().is_some_and(|found| found == extension)
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
        {
            out.insert(stem.to_owned());
        }
    }
    out
}

/// Gives a report type `with_load_error(message)`: its default (empty) report with `load_error`
/// set, used when the manifest or a package selection step fails before anything is examined.
/// The type must implement `Default` and have a `load_error: Option<String>` field.
macro_rules! impl_with_load_error {
    ($report:ty) => {
        impl $report {
            pub(crate) fn with_load_error(message: impl Into<String>) -> Self {
                Self {
                    load_error: Some(message.into()),
                    ..Self::default()
                }
            }
        }
    };
}

pub(crate) use impl_with_load_error;
