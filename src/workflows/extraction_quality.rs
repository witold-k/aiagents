// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Non-authoritative, protocol-independent warnings for source-backed facts.
//! These diagnostics do not modify facts or their review status.

use super::{FactReview, SourceFact};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(super) struct QualityFinding {
    path: String,
    fact_index: usize,
    kind: &'static str,
}

fn has_conditional_cue(text: &str) -> bool {
    text.split(|c: char| !c.is_alphabetic())
        .any(|word| matches!(word.to_ascii_lowercase().as_str(), "if" | "unless"))
}

fn joined_declarations(text: &str) -> bool {
    // A conservative signal for accidentally joined typed declarations,
    // not a parser or proof that the statement is malformed.
    const TYPES: [&str; 4] = ["int ", "float ", "binary ", "boolean "];
    TYPES.iter().any(|token| {
        text.match_indices(token).any(|(index, _)| {
            index > 0
                && text.as_bytes()[index - 1].is_ascii_lowercase()
                && text[index + token.len()..]
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_alphabetic())
        })
    })
}

pub(super) fn diagnose(facts: &BTreeMap<String, Vec<SourceFact>>) -> Vec<QualityFinding> {
    let mut findings = Vec::new();
    for (path, entries) in facts {
        for (fact_index, fact) in entries.iter().enumerate() {
            let mut push = |kind| {
                findings.push(QualityFinding {
                    path: path.clone(),
                    fact_index,
                    kind,
                })
            };
            if fact.review == FactReview::Unresolved {
                push("unresolved");
            }
            if joined_declarations(&fact.statement) {
                push("possible_joined_declarations");
            }
            if fact
                .evidence_quote
                .as_deref()
                .is_some_and(has_conditional_cue)
                && !has_conditional_cue(&fact.statement)
            {
                push("possible_lost_condition");
            }
        }
    }
    findings
}

#[cfg(test)]
#[path = "../../tests/workflows/extraction_quality_test.rs"]
mod tests;
