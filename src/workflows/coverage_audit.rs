// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Read-only, source-grounded replay count audit. Counts are diagnostics, not
//! a claim of semantic correctness or completeness of the published JSON.

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(super) struct CoverageAudit {
    schema_version: u32,
    candidate_facts: usize,
    distinct_statement_evidence_pairs: usize,
    explicitly_typed_candidates: usize,
    unresolved_candidates: usize,
    unresolved_candidate_facts: usize,
    statement_located: usize,
    statement_missing: usize,
    statement_exact_match: usize,
    statement_mismatch: usize,
    verified_parameter_assertions: usize,
    parameter_matched: usize,
    parameter_unmatched: usize,
    parameter_other: usize,
    checks: Vec<CoverageCheck>,
    caveats: Vec<&'static str>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CoverageCheck {
    name: &'static str,
    passed: bool,
}

impl CoverageAudit {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        candidate_facts: usize,
        distinct_pairs: usize,
        typed: usize,
        unresolved_candidates: usize,
        unresolved_facts: usize,
        locator_total: usize,
        located: usize,
        missing: usize,
        exact: usize,
        mismatch: usize,
        parameter_inventory: usize,
        parameter_unresolved: usize,
        matched: usize,
        unmatched: usize,
        other: usize,
    ) -> Self {
        let checks = vec![
            CoverageCheck { name: "distinct_pairs_not_greater_than_candidates", passed: distinct_pairs <= candidate_facts },
            CoverageCheck { name: "locator_total_matches_candidates", passed: locator_total == candidate_facts },
            CoverageCheck { name: "locator_partition", passed: located.checked_add(missing) == Some(locator_total) },
            CoverageCheck { name: "exact_within_located", passed: exact <= located },
            CoverageCheck { name: "mismatch_within_located", passed: mismatch <= located },
            CoverageCheck { name: "parameter_partition", passed: matched.checked_add(unmatched).and_then(|n| n.checked_add(other)) == Some(parameter_unresolved) },
            CoverageCheck { name: "parameter_inventory_matches_coverage", passed: parameter_inventory == matched },
            CoverageCheck { name: "unresolved_facts_within_candidates", passed: unresolved_facts <= candidate_facts },
        ];
        Self {
            schema_version: 1,
            candidate_facts,
            distinct_statement_evidence_pairs: distinct_pairs,
            explicitly_typed_candidates: typed,
            unresolved_candidates,
            unresolved_candidate_facts: unresolved_facts,
            statement_located: located,
            statement_missing: missing,
            statement_exact_match: exact,
            statement_mismatch: mismatch,
            verified_parameter_assertions: parameter_inventory,
            parameter_matched: matched,
            parameter_unmatched: unmatched,
            parameter_other: other,
            checks,
            caveats: vec![
                "A missing statement locator is not proof of missing source evidence.",
                "Exact source location does not establish semantic correctness.",
                "This audit does not compare staged facts with published JSON.",
                "Unresolved identity does not imply an extraction error.",
            ],
        }
    }

    pub(super) fn passed(&self) -> bool {
        self.checks.iter().all(|check| check.passed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> CoverageAudit {
        CoverageAudit::new(238, 235, 15, 10, 115, 238, 124, 114, 124, 0, 78, 95, 78, 13, 4)
    }

    #[test]
    fn openamip_replay_counts_are_consistent() {
        let audit = example();
        assert!(audit.passed());
        let json = serde_json::to_value(&audit).unwrap();
        assert_eq!(json["statement_missing"], 114);
        assert_eq!(json["parameter_unmatched"], 13);
    }

    #[test]
    fn detects_inconsistent_parameter_partition() {
        let audit = CoverageAudit::new(238, 235, 15, 10, 115, 238, 124, 114, 124, 0, 78, 95, 78, 12, 4);
        assert!(!audit.passed());
        assert!(audit.checks.iter().any(|c| c.name == "parameter_partition" && !c.passed));
    }

    #[test]
    fn detects_inconsistent_locator_partition() {
        let audit = CoverageAudit::new(238, 235, 15, 10, 115, 238, 124, 113, 124, 0, 78, 95, 78, 13, 4);
        assert!(!audit.passed());
    }
}
