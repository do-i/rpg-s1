//! Shared helpers for this crate's unit tests.

/// Expands to the AssetServer-logical path of one file in the shipped scenario package.
///
/// Tests that assert a resolved asset path route through this macro so the package location is
/// defined once per crate rather than repeated at every expectation.
///
/// Deliberately *not* used by [`crate::scenario_root`]'s own tests: those verify that
/// `ScenarioRoot` builds `scenarios/<key>/...` correctly, so their expectations must stay literal.
/// Asserting the resolver against a helper that encodes the same layout would prove nothing.
macro_rules! scenario_asset {
    ($scenario_relative_path:literal) => {
        concat!("scenarios/rusted_kingdoms/", $scenario_relative_path)
    };
}

/// Expands to an absolute path to one file in the shipped Rusted Kingdoms scenario package.
///
/// `include_str!`/`include_bytes!` need a literal path, so pinned content routes through this
/// macro to keep the package location defined once for the crate.
///
/// Anchored at `CARGO_MANIFEST_DIR` rather than a `../`-relative path, because `include_str!`
/// resolves relative paths against the *invoking* file — a relative form would need a different
/// prefix for every module depth.
macro_rules! scenario_file {
    ($scenario_relative_path:literal) => {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/scenarios/rusted_kingdoms/",
            $scenario_relative_path
        )
    };
}

/// Filesystem path to the shipped Rusted Kingdoms package, for tests that read or validate it.
///
/// Anchored at `CARGO_MANIFEST_DIR` so it holds regardless of the caller's module depth.
pub(crate) fn scenario_package_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios/rusted_kingdoms")
}
