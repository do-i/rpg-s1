//! Saveable cumulative player time.
//!
//! The source game measures a session with wall-clock time.  Consequently, a running session
//! includes time spent in field menus and any other application state; it is deliberately not
//! driven by Bevy's pausable virtual gameplay clock.  Callers supply wall-clock samples as
//! [`Duration`] values, which makes the clock boundary deterministic in tests and keeps actual
//! new-game, load, and save wiring out of this infrastructure task.

use std::time::Duration;

use bevy::prelude::Resource;

/// Cumulative, saveable player time measured in whole seconds.
///
/// `session_start` is intentionally in-memory only.  A new game or restored save will call
/// [`Self::start_session`] with a wall-clock sample, while save handling will call
/// [`Self::commit_session`].  Queries expose committed time only, matching the Python source.
#[derive(Clone, Debug, Default, Eq, PartialEq, Resource)]
pub struct Playtime {
    total_seconds: u64,
    session_start: Option<Duration>,
}

impl Playtime {
    /// Restores previously serialized whole seconds without starting a live session.
    pub const fn from_seconds(total_seconds: u64) -> Self {
        Self {
            total_seconds,
            session_start: None,
        }
    }

    /// Starts (or restarts) the uncommitted wall-clock session segment at `now`.
    pub fn start_session(&mut self, now: Duration) {
        self.session_start = Some(now);
    }

    /// Commits a wall-clock segment into the total and begins the next segment at `now`.
    ///
    /// Fractions of a second are discarded.  A sample earlier than the current segment start is
    /// ignored and leaves that start intact: a wall clock adjustment can neither subtract time
    /// nor create a new segment that later double-counts it.  Totals saturate at `u64::MAX`.
    pub fn commit_session(&mut self, now: Duration) {
        let Some(start) = self.session_start else {
            return;
        };
        let Some(elapsed) = now.checked_sub(start) else {
            return;
        };

        self.total_seconds = self.total_seconds.saturating_add(elapsed.as_secs());
        self.session_start = Some(now);
    }

    /// Returns committed whole seconds, excluding the active uncommitted segment.
    pub const fn total_seconds(&self) -> u64 {
        self.total_seconds
    }

    /// Returns the value to serialize, excluding the active uncommitted segment.
    pub const fn to_seconds(&self) -> u64 {
        self.total_seconds
    }

    /// Formats a total as `DDd HHh MMm SSs`.
    pub fn format(seconds: u64) -> String {
        let days = seconds / 86_400;
        let hours = (seconds % 86_400) / 3_600;
        let minutes = (seconds % 3_600) / 60;
        let seconds = seconds % 60;
        format!("{days:02}d {hours:02}h {minutes:02}m {seconds:02}s")
    }

    /// Formats the committed total for player-visible save-slot UI.
    pub fn display(&self) -> String {
        Self::format(self.total_seconds)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Playtime;

    #[test]
    fn default_is_zero_and_has_no_active_session() {
        let playtime = Playtime::default();

        assert_eq!(playtime.total_seconds(), 0);
        assert_eq!(playtime.to_seconds(), 0);
    }

    #[test]
    fn restored_seconds_are_preserved_until_a_new_segment_is_committed() {
        let mut playtime = Playtime::from_seconds(100);
        playtime.start_session(Duration::from_secs(10));
        playtime.commit_session(Duration::from_secs(210));

        assert_eq!(playtime.total_seconds(), 300);
    }

    #[test]
    fn commit_without_start_is_a_no_op() {
        let mut playtime = Playtime::default();
        playtime.commit_session(Duration::from_secs(3_600));

        assert_eq!(playtime.total_seconds(), 0);
    }

    #[test]
    fn commit_accumulates_whole_elapsed_seconds_and_resets_the_segment() {
        let mut playtime = Playtime::default();
        playtime.start_session(Duration::from_secs(10));
        playtime.commit_session(Duration::from_millis(1_010_900));
        playtime.commit_session(Duration::from_millis(1_510_900));

        assert_eq!(playtime.total_seconds(), 1_500);
    }

    #[test]
    fn queries_exclude_an_active_uncommitted_segment() {
        let mut playtime = Playtime::from_seconds(50);
        playtime.start_session(Duration::ZERO);

        assert_eq!(playtime.total_seconds(), 50);
        assert_eq!(playtime.to_seconds(), 50);
    }

    #[test]
    fn backward_clock_samples_are_ignored_without_resetting_the_segment() {
        let mut playtime = Playtime::default();
        playtime.start_session(Duration::from_secs(100));
        playtime.commit_session(Duration::from_secs(90));
        playtime.commit_session(Duration::from_secs(130));

        assert_eq!(playtime.total_seconds(), 30);
    }

    #[test]
    fn total_saturates_on_overflow() {
        let mut playtime = Playtime::from_seconds(u64::MAX);
        playtime.start_session(Duration::ZERO);
        playtime.commit_session(Duration::from_secs(1));

        assert_eq!(playtime.total_seconds(), u64::MAX);
    }

    #[test]
    fn format_matches_python_rules() {
        assert_eq!(Playtime::format(0), "00d 00h 00m 00s");
        assert_eq!(Playtime::format(3_600), "00d 01h 00m 00s");
        assert_eq!(Playtime::format(86_400), "01d 00h 00m 00s");
        assert_eq!(Playtime::format(367_200), "04d 06h 00m 00s");
        assert_eq!(Playtime::from_seconds(3_600).display(), "00d 01h 00m 00s");
    }
}
