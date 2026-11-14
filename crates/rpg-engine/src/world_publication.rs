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

/// Failure and spawn bookkeeping shared by the World layer states.
///
/// Each layer keeps its own fields and status enum and supplies the accessors; the defaults
/// hold the logic once. Readiness rules and handle lists stay per layer (`publication_status`
/// and `asset_ids` remain inherent), built from `publication_from` / `map_gated_publication`.
pub(crate) trait WorldLayerState {
    type Status: Copy + Eq;
    type Failure: std::fmt::Display;
    const SPAWNED: Self::Status;
    const FAILED: Self::Status;

    fn layer_map_id(&self) -> Option<&str>;
    fn layer_status(&self) -> Self::Status;
    fn layer_failure(&self) -> Option<&Self::Failure>;
    fn layer_status_and_failure_mut(&mut self) -> (&mut Self::Status, &mut Option<Self::Failure>);

    /// Marks the layer failed; the map id, handles and entities are left as they are.
    fn fail(&mut self, reason: impl Into<Self::Failure>) {
        let (status, failure) = self.layer_status_and_failure_mut();
        *status = Self::FAILED;
        *failure = Some(reason.into());
    }

    fn is_spawned_for(&self, map_id: &str) -> bool {
        self.layer_map_id() == Some(map_id) && self.layer_status() == Self::SPAWNED
    }

    fn failure_text(&self) -> Option<String> {
        self.layer_failure().map(ToString::to_string)
    }

    /// A recorded failure wins for any map; otherwise `ready` decides.
    fn publication_from(&self, ready: bool) -> PublicationStatus {
        if let Some(reason) = self.failure_text() {
            return PublicationStatus::Failed(reason);
        }
        if ready {
            PublicationStatus::Ready
        } else {
            PublicationStatus::Pending
        }
    }

    /// Another map's request is Pending even when it failed; only `map_id`'s failure counts.
    fn map_gated_publication(&self, map_id: &str) -> PublicationStatus {
        if self.layer_map_id() != Some(map_id) {
            return PublicationStatus::Pending;
        }
        self.publication_from(self.is_spawned_for(map_id))
    }
}

/// Per-layer fixtures for the shared publication test: each layer builds its own states.
#[cfg(test)]
pub(crate) trait PublicationFixture: Default + WorldLayerState {
    /// Text `failure_reason` renders as.
    const FAILURE: &'static str;
    fn failure_reason() -> Self::Failure;
    fn loading(map_id: &str) -> Self;
    fn spawned(map_id: &str) -> Self;
    /// The layer's own `publication_status` (the player's also takes the controlled member).
    fn probe_status(&self, map_id: &str) -> PublicationStatus;
}

#[cfg(test)]
mod tests {
    use super::{PublicationFixture, PublicationStatus};
    use crate::{
        tmx_ground_asset::StaticMapRenderState, world_actor::WorldActorState,
        world_encounter::WorldEncounterState, world_object::WorldObjectState,
        world_player::WorldPlayerSpawnState,
    };

    const A: &str = "map_a";
    const B: &str = "map_b";

    /// What one state reports for map A and map B, whether it is spawned for A, and its failure.
    fn observe<S: PublicationFixture>(
        state: &S,
    ) -> (PublicationStatus, PublicationStatus, bool, Option<String>) {
        (
            state.probe_status(A),
            state.probe_status(B),
            state.is_spawned_for(A),
            state.failure_text(),
        )
    }

    /// `gated` layers report Pending for another map even while failed.
    fn drive<S: PublicationFixture>(gated: bool) {
        use PublicationStatus::{Failed, Pending, Ready};
        let failed = || Failed(S::FAILURE.to_owned());
        let reason = Some(S::FAILURE.to_owned());

        assert_eq!(observe(&S::default()), (Pending, Pending, false, None));
        assert_eq!(observe(&S::loading(A)), (Pending, Pending, false, None));

        let mut state = S::loading(A);
        state.fail(S::failure_reason());
        let other = if gated { Pending } else { failed() };
        assert_eq!(observe(&state), (failed(), other, false, reason.clone()));

        // A failure before any map was recorded (missing current map, invalid path).
        let mut state = S::default();
        state.fail(S::failure_reason());
        let mapless = if gated { Pending } else { failed() };
        assert_eq!(
            observe(&state),
            (
                mapless,
                if gated { Pending } else { failed() },
                false,
                reason
            )
        );

        assert_eq!(observe(&S::spawned(A)), (Ready, Pending, true, None));
    }

    #[test]
    fn every_world_layer_reports_publication_through_the_same_sequence() {
        drive::<StaticMapRenderState>(false);
        drive::<WorldPlayerSpawnState>(false);
        drive::<WorldActorState>(true);
        drive::<WorldObjectState>(true);
        drive::<WorldEncounterState>(false);
    }
}
