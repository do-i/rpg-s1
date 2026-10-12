//! Shared outcomes and ordering for a World map publication transaction.

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState, UntypedAssetId},
    prelude::*,
};

#[derive(Debug, PartialEq)]
pub(crate) enum PublicationStatus {
    Pending,
    Ready,
    Failed(String),
}

/// Loading runs before the barrier; gameplay and persistence see its final lock state.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum WorldPublicationSet {
    Load,
    Barrier,
}

pub(crate) fn asset_failed(server: &AssetServer, id: impl Into<UntypedAssetId>) -> bool {
    let id = id.into();
    matches!(server.load_state(id), LoadState::Failed(_))
        || matches!(
            server.recursive_dependency_load_state(id),
            RecursiveDependencyLoadState::Failed(_)
        )
}
