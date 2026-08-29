//! Compares a scenario validation report against a checked-in list of accepted diagnostics.
//!
//! A handful of validator diagnostics are accepted inherited source debt that must *not* be
//! driven to zero by inventing unlocks, maps, or flags. That leaves bare validation permanently
//! exiting non-zero, which is why it could not be a CI gate and why a broken title cursor
//! reference once shipped unnoticed.
//!
//! A baseline resolves both halves. The accepted diagnostics are enumerated in a file; the run is
//! clean only when the report matches that file exactly. A *new* diagnostic fails, so regressions
//! cannot ship. A diagnostic that no longer appears also fails, so the file cannot quietly rot into
//! a list of things that were fixed years ago -- paying down debt is expected to include deleting
//! its line.
//!
//! Comparison is by the exact rendered report line, so the baseline reads as the validator's own
//! output and needs no second format to learn.

use std::collections::BTreeMap;

/// The outcome of checking reported diagnostics against the accepted set.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BaselineComparison {
    /// Reported diagnostics that the baseline accounts for.
    pub matched: usize,
    /// Reported diagnostics the baseline does not list -- regressions.
    pub unexpected: Vec<String>,
    /// Baseline entries the report no longer produces -- debt that was paid but not deleted.
    pub resolved: Vec<String>,
}

impl BaselineComparison {
    /// True when the report and the baseline describe the same multiset of diagnostics.
    pub fn is_clean(&self) -> bool {
        self.unexpected.is_empty() && self.resolved.is_empty()
    }
}

/// Reads accepted diagnostic lines from a baseline file's contents.
///
/// Blank lines and `#` comments are dropped so the file can carry its own rationale; every other
/// line is one accepted diagnostic, trimmed of surrounding whitespace.
pub fn parse_baseline(contents: &str) -> Vec<String> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// Compares rendered report lines against accepted baseline lines.
///
/// Both sides are treated as multisets: two identical diagnostics require two baseline lines. The
/// returned lists are sorted so CI output is stable regardless of report ordering.
pub fn compare_to_baseline(reported: &[String], baseline: &[String]) -> BaselineComparison {
    let mut remaining = counts_of(baseline);
    let mut unexpected = Vec::new();
    let mut matched = 0;

    for line in reported {
        match remaining.get_mut(line) {
            Some(count) if *count > 0 => {
                *count -= 1;
                matched += 1;
            }
            _ => unexpected.push(line.clone()),
        }
    }

    let mut resolved = remaining
        .into_iter()
        .flat_map(|(line, count)| std::iter::repeat_n(line, count))
        .collect::<Vec<_>>();

    unexpected.sort();
    resolved.sort();
    BaselineComparison {
        matched,
        unexpected,
        resolved,
    }
}

fn counts_of(lines: &[String]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for line in lines {
        *counts.entry(line.clone()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn a_report_matching_the_baseline_exactly_is_clean() {
        let comparison = compare_to_baseline(
            &lines(&["error a [x] one", "warning b [y] two"]),
            &lines(&["error a [x] one", "warning b [y] two"]),
        );

        assert!(comparison.is_clean());
        assert_eq!(comparison.matched, 2);
    }

    #[test]
    fn a_diagnostic_missing_from_the_baseline_is_reported_as_unexpected() {
        let comparison = compare_to_baseline(
            &lines(&["error a [x] one", "error c [z] three"]),
            &lines(&["error a [x] one"]),
        );

        assert!(!comparison.is_clean());
        assert_eq!(comparison.unexpected, lines(&["error c [z] three"]));
        assert!(comparison.resolved.is_empty());
        assert_eq!(comparison.matched, 1);
    }

    #[test]
    fn a_baseline_entry_the_report_no_longer_produces_is_reported_as_resolved() {
        let comparison = compare_to_baseline(
            &lines(&["error a [x] one"]),
            &lines(&["error a [x] one", "error c [z] three"]),
        );

        assert!(!comparison.is_clean());
        assert!(comparison.unexpected.is_empty());
        assert_eq!(comparison.resolved, lines(&["error c [z] three"]));
    }

    #[test]
    fn identical_diagnostics_are_matched_one_baseline_line_each() {
        let twice = lines(&["error a [x] one", "error a [x] one"]);

        let under = compare_to_baseline(&twice, &lines(&["error a [x] one"]));
        assert_eq!(under.unexpected, lines(&["error a [x] one"]));
        assert_eq!(under.matched, 1);

        let over = compare_to_baseline(&lines(&["error a [x] one"]), &twice);
        assert_eq!(over.resolved, lines(&["error a [x] one"]));
    }

    #[test]
    fn unexpected_and_resolved_entries_are_sorted_for_stable_ci_output() {
        let comparison = compare_to_baseline(
            &lines(&["error z [x] last", "error b [x] first"]),
            &lines(&["error y [x] gone", "error a [x] also gone"]),
        );

        assert_eq!(
            comparison.unexpected,
            lines(&["error b [x] first", "error z [x] last"])
        );
        assert_eq!(
            comparison.resolved,
            lines(&["error a [x] also gone", "error y [x] gone"])
        );
    }

    #[test]
    fn comments_and_blank_lines_are_not_accepted_diagnostics() {
        let parsed = parse_baseline(
            "# why these are accepted\n\
             \n\
             error a [x] one\n\
             \x20  error b [y] two  \n\
             # trailing note\n",
        );

        assert_eq!(parsed, lines(&["error a [x] one", "error b [y] two"]));
    }

    #[test]
    fn an_empty_baseline_demands_a_clean_report() {
        assert!(compare_to_baseline(&[], &[]).is_clean());
        assert_eq!(
            compare_to_baseline(&lines(&["error a [x] one"]), &[]).unexpected,
            lines(&["error a [x] one"])
        );
    }
}
