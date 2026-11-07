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


#[derive(Debug, Serialize)]
pub(super) struct MissingLocatorDetail {
    candidate_id: String,
    proposed_name: String,
    fact_index: usize,
    segment_id: String,
    statement: String,
    evidence_quote: Option<String>,
    source_window: Option<usize>,
    source_byte_offset: Option<usize>,
    source_locator_ambiguous: bool,
    exact_statement_occurrences_in_window: Option<usize>,
    exact_quote_occurrences_in_window: Option<usize>,
    reason: &'static str,
}

fn window_index(candidate_id: &str) -> Option<usize> {
    candidate_id
        .strip_prefix("window-")?
        .split('-')
        .next()?
        .parse()
        .ok()
}

/// Enumerate missing statement locators without inferring a replacement position.
/// A statement can be a valid paraphrase even when it does not occur verbatim.
pub(super) fn missing_locator_details(
    candidates: &[super::message_identity::MessageCandidate],
    windows: &[&str],
) -> Vec<MissingLocatorDetail> {
    let mut result = Vec::new();
    for candidate in candidates {
        let window_index = window_index(&candidate.id);
        let window = window_index.and_then(|index| windows.get(index)).copied();
        for (fact_index, fact) in candidate.facts.iter().enumerate() {
            if fact.statement_byte_offset.is_some() {
                continue;
            }
            let statement_hits = window.map(|source| {
                if fact.statement.is_empty() { 0 } else { source.matches(&fact.statement).count() }
            });
            let quote_hits = window.and_then(|source| {
                fact.evidence_quote.as_ref().map(|quote| {
                    if quote.is_empty() { 0 } else { source.matches(quote).count() }
                })
            });
            let reason = if window.is_none() {
                "source_window_unavailable"
            } else if fact.source_locator_ambiguous {
                "ambiguous_source_locator"
            } else if fact.statement.is_empty() {
                "empty_statement"
            } else {
                match statement_hits {
                    Some(0) => "statement_not_verbatim_in_window",
                    Some(1) => "unique_statement_without_locator",
                    Some(_) => "repeated_statement_in_window",
                    None => "source_window_unavailable",
                }
            };
            result.push(MissingLocatorDetail {
                candidate_id: candidate.id.clone(),
                proposed_name: candidate.proposed_name.clone(),
                fact_index,
                segment_id: fact.segment_id.clone(),
                statement: fact.statement.clone(),
                evidence_quote: fact.evidence_quote.clone(),
                source_window: window_index,
                source_byte_offset: fact.source_byte_offset,
                source_locator_ambiguous: fact.source_locator_ambiguous,
                exact_statement_occurrences_in_window: statement_hits,
                exact_quote_occurrences_in_window: quote_hits,
                reason,
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_locator_reasons_preserve_source_evidence() {
        use super::super::message_identity::{CandidateFact, MessageCandidate};
        let candidate = MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "unknown".into(),
            facts: [
                ("alpha", Some("alpha"), false),
                ("beta", Some("beta"), false),
                ("paraphrase", Some("alpha"), false),
                ("alpha", Some("alpha"), true),
            ]
            .into_iter()
            .map(|(statement, quote, ambiguous)| CandidateFact {
                statement: statement.into(),
                segment_id: "S1".into(),
                evidence_quote: quote.map(str::to_owned),
                source_byte_offset: Some(0),
                source_locator_ambiguous: ambiguous,
                statement_byte_offset: None,
            })
            .collect(),
        };
        let details = missing_locator_details(&[candidate], &["alpha beta alpha"]);
        let json = serde_json::to_value(&details).unwrap();
        assert_eq!(json[0]["reason"], "repeated_statement_in_window");
        assert_eq!(json[1]["reason"], "unique_statement_without_locator");
        assert_eq!(json[2]["reason"], "statement_not_verbatim_in_window");
        assert_eq!(json[3]["reason"], "ambiguous_source_locator");
        assert_eq!(json[0]["exact_quote_occurrences_in_window"], 2);
        assert_eq!(json[0]["source_byte_offset"], 0);
    }

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
