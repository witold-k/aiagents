// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Diagnostic, language-neutral protocol model staging.
//! This is an inventory of hypotheses and source anchors, not a wire schema.

use serde::Serialize;
use super::message_identity::{source_window_index, MessageCandidate};

#[derive(Debug, Serialize)]
pub(super) struct ProtocolModel {
    schema_version: u32,
    status: &'static str,
    messages: Vec<MessageHypothesis>,
    caveats: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct MessageHypothesis {
    candidate_id: String,
    proposed_name: String,
    identity_status: &'static str,
    assertions: Vec<Assertion>,
    parameter_hints: Vec<ParameterHint>,
}

#[derive(Debug, Serialize)]
struct Assertion {
    statement: String,
    segment_id: String,
    evidence_quote: Option<String>,
    source_window: Option<usize>,
    source_byte_offset: Option<usize>,
    source_quote_status: &'static str,
}

#[derive(Debug, Serialize)]
struct ParameterHint {
    name: String,
    origin_assertion: usize,
    status: &'static str,
    value_type: Option<String>,
    required: Option<bool>,
    constraints: Vec<String>,
}

/// Recognize only an explicit key=value token. No guessed types, message
/// ownership or prose-based field reconstruction.
fn assignment_key(text: &str) -> Option<&str> {
    text.split_whitespace().find_map(|word| {
        let token = word.trim_matches(|c: char| matches!(c, '|' | '`' | ',' | ';' | '*' | '(' | ')'));
        let (key, value) = token.split_once('=')?;
        if key.is_empty() || value.is_empty()
            || !key.starts_with(|c: char| c.is_ascii_alphabetic())
            || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return None;
        }
        Some(key)
    })
}

pub(super) fn stage(candidates: &[MessageCandidate], windows: &[&str]) -> ProtocolModel {
    let messages = candidates.iter().map(|candidate| {
        let window_index = source_window_index(&candidate.id);
        let mut assertions = Vec::with_capacity(candidate.facts.len());
        let mut parameter_hints = Vec::new();
        for (index, fact) in candidate.facts.iter().enumerate() {
            let anchored = window_index
                .and_then(|i| windows.get(i))
                .zip(fact.source_byte_offset)
                .zip(fact.evidence_quote.as_deref())
                .filter(|(_, quote)| !quote.is_empty())
                .and_then(|((window, start), quote)| {
                    start.checked_add(quote.len())
                        .and_then(|end| window.get(start..end))
                        .map(|actual| actual == quote)
                })
                .unwrap_or(false) && !fact.source_locator_ambiguous;
            assertions.push(Assertion {
                statement: fact.statement.clone(),
                segment_id: fact.segment_id.clone(),
                evidence_quote: fact.evidence_quote.clone(),
                source_window: window_index,
                source_byte_offset: fact.source_byte_offset,
                source_quote_status: if anchored { "exact" } else { "unverified" },
            });
            // Hints come from the literal source quote, never from a paraphrase.
            // Even exact lexical hints are not validated parameter definitions.
            if anchored && let Some(key) = fact.evidence_quote.as_deref().and_then(assignment_key) {
                parameter_hints.push(ParameterHint {
                    name: key.to_owned(),
                    origin_assertion: index,
                    status: "lexical_hypothesis",
                    value_type: None,
                    required: None,
                    constraints: Vec::new(),
                });
            }
        }
        MessageHypothesis {
            candidate_id: candidate.id.clone(),
            proposed_name: candidate.proposed_name.clone(),
            identity_status: "unverified_candidate",
            assertions,
            parameter_hints,
        }
    }).collect();
    ProtocolModel {
        schema_version: 1,
        status: "diagnostic_hypotheses_only",
        messages,
        caveats: vec![
            "Candidates are not canonical messages; repeated names do not imply identity.",
            "Parameter hints are literal key=value syntax, not verified fields or message ownership.",
            "Types, cardinality, constraints, defaults and protocol behavior are not inferred.",
            "An exact source quotation does not establish the semantic truth of a statement.",
            "This model is not suitable for automatic code generation.",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::message_identity::CandidateFact;

    fn candidate(quote: &str, offset: Option<usize>, ambiguous: bool) -> MessageCandidate {
        MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "unverified".into(),
            facts: vec![CandidateFact {
                statement: "candidate fact".into(),
                segment_id: "segment-0".into(),
                evidence_quote: Some(quote.into()),
                source_byte_offset: offset,
                source_locator_ambiguous: ambiguous,
                statement_byte_offset: None,
            }],
        }
    }

    #[test]
    fn exact_assignment_is_only_a_lexical_hint() {
        let report = stage(&[candidate("replyTo=getExtAntStatus", Some(0), false)], &["replyTo=getExtAntStatus"]);
        let message = &report.messages[0];
        assert_eq!(message.assertions[0].source_quote_status, "exact");
        assert_eq!(message.parameter_hints[0].name, "replyTo");
        assert_eq!(message.parameter_hints[0].status, "lexical_hypothesis");
        assert_eq!(message.parameter_hints[0].value_type, None);
    }

    #[test]
    fn missing_or_ambiguous_quote_cannot_produce_parameter_hint() {
        for (offset, ambiguous) in [(None, false), (Some(0), true), (Some(1), false)] {
            let report = stage(&[candidate("statusCode=18", offset, ambiguous)], &["statusCode=18"]);
            assert!(report.messages[0].parameter_hints.is_empty());
            assert_eq!(report.messages[0].assertions[0].source_quote_status, "unverified");
        }
    }

    #[test]
    fn prose_and_table_caption_are_not_parameter_hints() {
        let report = stage(&[candidate("Table A-5. Transceiver", Some(0), false)], &["Table A-5. Transceiver"]);
        assert!(report.messages[0].parameter_hints.is_empty());
    }
}
