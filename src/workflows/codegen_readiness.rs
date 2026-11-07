// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Read-only inventory of prerequisites for generating protocol code.
//! Candidate names and lexical clues are not promoted to a canonical schema.

use serde::Serialize;
use super::message_identity::{MessageCandidate, source_window_index};

#[derive(Debug, Serialize)]
pub(super) struct CodegenReadinessReport {
    schema_version: u32,
    status: &'static str,
    candidates: usize,
    facts: usize,
    exact_source_anchored_facts: usize,
    unanchored_or_ambiguous_facts: usize,
    candidates_with_no_facts: usize,
    candidate_inventory: Vec<CandidateReadiness>,
    unresolved_prerequisites: Vec<&'static str>,
    caveats: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct CandidateReadiness {
    candidate_id: String,
    proposed_name: String,
    facts: usize,
    exact_source_anchored_facts: usize,
    identity_status: &'static str,
    field_schema_status: &'static str,
}

fn exact_anchor(candidate: &MessageCandidate, windows: &[&str], fact: &super::message_identity::CandidateFact) -> bool {
    if fact.source_locator_ambiguous { return false; }
    let Some(index) = source_window_index(&candidate.id) else { return false };
    let Some(window) = windows.get(index) else { return false };
    let Some(quote) = fact.evidence_quote.as_deref().filter(|q| !q.is_empty()) else { return false };
    let Some(start) = fact.source_byte_offset else { return false };
    start.checked_add(quote.len())
        .and_then(|end| window.get(start..end))
        == Some(quote)
}

pub(super) fn audit(candidates: &[MessageCandidate], windows: &[&str]) -> CodegenReadinessReport {
    let mut facts = 0;
    let mut anchored = 0;
    let mut empty = 0;
    let mut inventory = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if candidate.facts.is_empty() { empty += 1; }
        let count = candidate.facts.iter().filter(|fact| exact_anchor(candidate, windows, fact)).count();
        facts += candidate.facts.len();
        anchored += count;
        inventory.push(CandidateReadiness {
            candidate_id: candidate.id.clone(),
            proposed_name: candidate.proposed_name.clone(),
            facts: candidate.facts.len(),
            exact_source_anchored_facts: count,
            identity_status: "unverified_candidate",
            field_schema_status: "not_derived",
        });
    }
    CodegenReadinessReport {
        schema_version: 1,
        status: "not_ready_for_automatic_code_generation",
        candidates: candidates.len(),
        facts,
        exact_source_anchored_facts: anchored,
        unanchored_or_ambiguous_facts: facts - anchored,
        candidates_with_no_facts: empty,
        candidate_inventory: inventory,
        unresolved_prerequisites: vec![
            "canonical_message_identities",
            "message_discriminators_and_wire_encoding",
            "field_names_types_and_cardinality",
            "field_constraints_defaults_and_optional_rules",
            "conditional_relationships_and_protocol_state",
            "parser_serializer_error_behavior",
        ],
        caveats: vec![
            "This is a prerequisite inventory, not a generated schema or a completeness score.",
            "An exact source quote validates its location, not the truth of its interpretation.",
            "Candidate names are not canonical message identities.",
            "Missing prerequisites mean not established by this audit, not necessarily absent from the source document.",
            "No message ownership, type, cardinality, or protocol behavior is inferred.",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::message_identity::CandidateFact;

    #[test]
    fn exact_quote_is_not_treated_as_validated_schema() {
        let source = "statusCode=18";
        let candidate = MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "status".into(),
            facts: vec![CandidateFact {
                statement: "status code".into(),
                segment_id: "segment".into(),
                evidence_quote: Some(source.into()),
                source_byte_offset: Some(0),
                source_locator_ambiguous: false,
                statement_byte_offset: None,
            }],
        };
        let report = audit(&[candidate], &[source]);
        assert_eq!(report.exact_source_anchored_facts, 1);
        assert_eq!(report.unanchored_or_ambiguous_facts, 0);
        assert_eq!(report.candidate_inventory[0].field_schema_status, "not_derived");
    }

    #[test]
    fn ambiguous_anchor_is_not_counted() {
        let candidate = MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "status".into(),
            facts: vec![CandidateFact {
                statement: "status code".into(),
                segment_id: "segment".into(),
                evidence_quote: Some("statusCode=18".into()),
                source_byte_offset: Some(0),
                source_locator_ambiguous: true,
                statement_byte_offset: None,
            }],
        };
        let report = audit(&[candidate], &["statusCode=18"]);
        assert_eq!(report.exact_source_anchored_facts, 0);
        assert_eq!(report.unanchored_or_ambiguous_facts, 1);
    }
}
