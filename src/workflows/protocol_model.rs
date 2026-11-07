// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Diagnostic, language-neutral protocol model staging.
//! This is an inventory of hypotheses and source anchors, not a wire schema.

use super::message_identity::{MessageCandidate, source_window_index};
use serde::Serialize;

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
        let token =
            word.trim_matches(|c: char| matches!(c, '|' | '`' | ',' | ';' | '*' | '(' | ')'));
        let (key, value) = token.split_once('=')?;
        if key.is_empty()
            || value.is_empty()
            || !key.starts_with(|c: char| c.is_ascii_alphabetic())
            || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return None;
        }
        Some(key)
    })
}

pub(super) fn stage(candidates: &[MessageCandidate], windows: &[&str]) -> ProtocolModel {
    let messages = candidates
        .iter()
        .map(|candidate| {
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
                        start
                            .checked_add(quote.len())
                            .and_then(|end| window.get(start..end))
                            .map(|actual| actual == quote)
                    })
                    .unwrap_or(false)
                    && !fact.source_locator_ambiguous;
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
                if anchored
                    && let Some(key) = fact.evidence_quote.as_deref().and_then(assignment_key)
                {
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
        })
        .collect();
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
#[path = "../../tests/workflows/protocol_model_test.rs"]
mod tests;
