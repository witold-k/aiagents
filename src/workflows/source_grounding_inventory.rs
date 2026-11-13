// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Read-only source-grounded triage. Never changes message identity or published facts.
use super::message_identity::{CandidateIdentity, MessageCandidate, classify_candidate};
use serde_json::{Value, json};

fn exact_span(source: &str, start: usize, quote: &str) -> Option<(usize, usize)> {
    let end = start.checked_add(quote.len())?;
    (source.get(start..end) == Some(quote)).then_some((start, end))
}

/// Report evidence positions, not positions of paraphrased statements.
/// A semantic category is a hypothesis and never a destination or ownership claim.
pub(super) fn build_inventory(
    candidates: &[MessageCandidate],
    windows: &[&str],
    starts: &[usize],
    document: &str,
) -> Value {
    let mut items = Vec::new();
    let mut located = 0usize;
    let mut unlocated = 0usize;
    for candidate in candidates {
        let unresolved = matches!(classify_candidate(candidate), CandidateIdentity::Unresolved);
        if !unresolved { continue; }
        let window_id = candidate.id.strip_prefix("coverage-").unwrap_or(&candidate.id)
            .strip_prefix("window-")
            .and_then(|rest| rest.split_once("-candidate-"))
            .and_then(|(id, _)| id.parse::<usize>().ok());
        for fact in &candidate.facts {
            let window = window_id.and_then(|i| windows.get(i).zip(starts.get(i)));
            let evidence = fact.evidence_quote.as_deref().unwrap_or("");
            let anchor = if fact.source_locator_ambiguous || evidence.is_empty() {
                None
            } else {
                window.and_then(|(source, base)| {
                    fact.source_byte_offset.and_then(|offset| {
                        exact_span(source, offset, evidence).and_then(|_| {
                            base.checked_add(offset).and_then(|absolute| {
                                exact_span(document, absolute, evidence)
                            })
                        })
                    })
                })
            };
            let (location_status, source_span, source_line, section_hint, category_hint) =
                if let Some((start, end)) = anchor {
                    located += 1;
                    let line = document[..start].bytes().filter(|b| *b == b'\n').count() + 1;
                    let preceding = &document[..start];
                    let section = preceding.lines().rev()
                        .find(|line| line.trim_start().starts_with('#'))
                        .map(|line| line.trim().to_owned());
                    let evidence_line = document[start..].lines().next().unwrap_or("");
                    let category = if evidence_line.contains('|') {
                        "possible_table_or_parameter"
                    } else if evidence_line.trim_start().starts_with("Type:") {
                        "possible_message_declaration"
                    } else {
                        "unclassified"
                    };
                    ("exact_evidence", Some(json!({"start":start,"end":end})), Some(line), section, category)
                } else {
                    unlocated += 1;
                    ("unlocated_or_ambiguous", None, None, None, "unclassified")
                };
            items.push(json!({
                "candidate_id": candidate.id,
                "segment_id": fact.segment_id,
                "statement": fact.statement,
                "evidence_quote": fact.evidence_quote,
                "location_status": location_status,
                "normalized_source_span": source_span,
                "normalized_source_line": source_line,
                "nearest_heading_hint": section_hint,
                "semantic_category_hint": category_hint,
                "semantic_category_verified": false,
                "message_ownership_verified": false,
                "source_locator_ambiguous": fact.source_locator_ambiguous
            }));
        }
    }
    json!({
        "version": 1,
        "read_only": true,
        "coordinate_system": "normalized_document_utf8_bytes",
        "located_exact_evidence": located,
        "unlocated_or_ambiguous": unlocated,
        "items": items
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::message_identity::CandidateFact;

    #[test]
    fn anchored_paraphrase_is_located_by_evidence_not_statement() {
        let document = "## Rules\n| parameter | description |\n";
        let candidate = MessageCandidate {
            id: "window-0-candidate-0".to_owned(),
            proposed_name: "unknown".to_owned(),
            facts: vec![CandidateFact {
                statement: "A paraphrase absent from source".to_owned(),
                segment_id: "S1".to_owned(),
                evidence_quote: Some("parameter".to_owned()),
                source_byte_offset: Some(11),
                source_locator_ambiguous: false,
                statement_byte_offset: None,
            }],
        };
        let result = build_inventory(&[candidate], &[document], &[0], document);
        assert_eq!(result["located_exact_evidence"], 1);
        assert_eq!(result["items"][0]["semantic_category_verified"], false);
    }

    #[test]
    fn ambiguous_source_does_not_gain_an_anchor() {
        let document = "Type: G\nType: G\n";
        let candidate = MessageCandidate {
            id: "window-0-candidate-0".to_owned(),
            proposed_name: "unknown".to_owned(),
            facts: vec![CandidateFact {
                statement: "anything".to_owned(),
                segment_id: "S1".to_owned(),
                evidence_quote: Some("Type: G".to_owned()),
                source_byte_offset: Some(0),
                source_locator_ambiguous: true,
                statement_byte_offset: None,
            }],
        };
        let result = build_inventory(&[candidate], &[document], &[0], document);
        assert_eq!(result["located_exact_evidence"], 0);
    }
}
