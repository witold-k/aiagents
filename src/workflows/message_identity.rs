// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! P4 staging model: collect names and facts without equating names with identity.
//! Consolidation must use document-wide source evidence, not string similarity.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct MessageCandidate {
    /// Stable within a collection; never derived from a suggested filename.
    pub id: String,
    /// Original, potentially free-form model/source label; not an identity.
    pub proposed_name: String,
    /// Source-attributed assertions pending semantic review.
    pub facts: Vec<CandidateFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct CandidateFact {
    pub statement: String,
    pub segment_id: String,
    pub evidence_quote: Option<String>,
    /// Exact byte position in the original source chunk; None in legacy checkpoints.
    #[serde(default)]
    pub source_byte_offset: Option<usize>,
    /// A recovery window matched multiple parent positions; never guess.
    #[serde(default)]
    pub source_locator_ambiguous: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg(test)]
pub(super) enum IdentityDecision {
    /// A source-grounded equivalence, not just matching normalized names.
    Merge {
        left: String,
        right: String,
        reason: String,
    },
    /// Distinct protocol identities, even if their human-readable names overlap.
    Split {
        left: String,
        right: String,
        reason: String,
    },
    /// Insufficient evidence: preserve both candidates and their facts.
    Unresolved {
        left: String,
        right: String,
        reason: String,
    },
}

/// Stage one candidate per extracted file without coalescing by label.
/// IDs include the source window and response file index to prevent accidental
/// collisions between identically named candidates in different windows.
#[cfg(test)]
pub(super) fn collect_candidates(
    source_window: usize,
    files: impl IntoIterator<Item = (String, Vec<CandidateFact>)>,
) -> Vec<MessageCandidate> {
    files
        .into_iter()
        .enumerate()
        .map(|(index, (proposed_name, facts))| MessageCandidate {
            id: format!("window-{source_window}-candidate-{index}"),
            proposed_name,
            facts,
        })
        .collect()
}

/// Parse source-backed facts independently of the model's proposed message
/// identity. Unknown names are retained, but fabricated segment locators,
/// contradictory explicit type labels and malformed paths are not accepted.
/// This is staging only: candidates must not become canonical output yet.
#[cfg(test)]
pub(super) fn parse_candidates(
    response: &str,
    source: &str,
    source_window: usize,
    offset: usize,
) -> Vec<MessageCandidate> {
    parse_candidates_in_parent(response, source, source, source_window, offset)
}

/// The model sees a possibly split recovery window, but checkpoint locators
/// always refer to its original parent chunk. Ambiguous parent placements
/// are deliberately left unresolved rather than guessed.
pub(super) fn parse_candidates_in_parent(
    response: &str,
    source: &str,
    parent_source: &str,
    source_window: usize,
    offset: usize,
) -> Vec<MessageCandidate> {
    let parent_start = if source == parent_source {
        Some(0)
    } else {
        let mut positions = parent_source.match_indices(source);
        match (positions.next(), positions.next()) {
            (Some((at, _)), None) => Some(at),
            _ => None,
        }
    };
    let normalized = crate::utils::stringutils::strip_code_fences(response.trim());
    let Ok(parsed) = serde_json::from_str::<super::SegmentedWindow>(normalized.trim()) else {
        return Vec::new();
    };
    let segments = super::evidence_segments(source);
    parsed
        .files
        .into_iter()
        .enumerate()
        .filter_map(|(index, file)| {
            if !super::valid_spec_path(&file.path) {
                return None;
            }
            let facts: Vec<CandidateFact> = file
                .facts
                .into_iter()
                .filter_map(|fact| {
                    let statement = fact.statement.trim();
                    if statement.is_empty() || statement.chars().count() > 4_000 {
                        return None;
                    }
                    let segment = segments
                        .iter()
                        .find(|segment| segment.id == fact.segment_id)?;
                    if segment.text.trim().is_empty() {
                        return None;
                    }
                    let evidence_type = super::explicit_type_label(&segment.text)
                        .or_else(|| super::section_type_for_line(source, segment.line_index));
                    if let (Some(stated), Some(evidenced)) =
                        (super::explicit_type_label(statement), evidence_type)
                        && stated != evidenced
                    {
                        return None;
                    }
                    Some(CandidateFact {
                        statement: statement.to_owned(),
                        segment_id: fact.segment_id,
                        evidence_quote: Some(segment.text.clone()),
                        source_byte_offset: parent_start.map(|start| start + segment.byte_offset),
                        source_locator_ambiguous: parent_start.is_none(),
                    })
                })
                .collect();
            if facts.is_empty() {
                return None;
            }
            Some(MessageCandidate {
                id: format!("window-{source_window}-candidate-{}", offset + index),
                proposed_name: file.path,
                facts,
            })
        })
        .collect()
}

/// Conservative document-wide classification. Only an explicit, consistent
/// source type is sufficient to assign a candidate to a canonical identity.
/// The model-proposed filename is never identity evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CandidateIdentity {
    Resolved(String),
    Unresolved,
}

pub(super) fn classify_candidate(candidate: &MessageCandidate) -> CandidateIdentity {
    let mut declared = None;
    for fact in &candidate.facts {
        let Some(quote) = fact.evidence_quote.as_deref() else {
            continue;
        };
        let Some(letter) = super::explicit_type_label(quote.trim().trim_start_matches('#').trim())
        else {
            continue;
        };
        if declared.is_some_and(|previous| previous != letter) {
            return CandidateIdentity::Unresolved;
        }
        declared = Some(letter);
    }
    match declared {
        Some(letter) if letter.is_ascii_uppercase() => CandidateIdentity::Resolved(format!(
            "messages/upper_{}.md",
            (letter as char).to_ascii_lowercase()
        )),
        Some(letter) => CandidateIdentity::Resolved(format!("messages/{}.md", letter as char)),
        None => CandidateIdentity::Unresolved,
    }
}

pub(super) fn identity_counts(candidates: &[MessageCandidate]) -> (usize, usize, usize) {
    let mut resolved = 0;
    let mut unresolved = 0;
    let mut unresolved_facts = 0;
    for candidate in candidates {
        match classify_candidate(candidate) {
            CandidateIdentity::Resolved(_) => resolved += 1,
            CandidateIdentity::Unresolved => {
                unresolved += 1;
                unresolved_facts += candidate.facts.len();
            }
        }
    }
    (resolved, unresolved, unresolved_facts)
}

/// Count distinct source-backed assertions across all proposed message identities.
/// This is an inventory, not an identity merge: a repeated assertion can still
/// belong to multiple candidates. The evidence text is part of the key so
/// similar wording with different provenance is not silently collapsed.
pub(super) fn distinct_source_fact_count(candidates: &[MessageCandidate]) -> usize {
    let mut distinct = std::collections::BTreeSet::new();
    for candidate in candidates {
        for fact in &candidate.facts {
            distinct.insert((
                fact.statement.trim(),
                fact.evidence_quote.as_deref().unwrap_or(""),
            ));
        }
    }
    distinct.len()
}

/// A candidate ID encodes its original extraction/coverage source window.
/// Never derive provenance from the model's suggested filename.
pub(super) fn source_window_index(id: &str) -> Option<usize> {
    let suffix = id.strip_prefix("coverage-").unwrap_or(id);
    let rest = suffix.strip_prefix("window-")?;
    let (window, _) = rest.split_once("-candidate-")?;
    window.parse().ok()
}

/// Resolve a type within one uninterrupted table-like source block.
/// Unlike the legacy twelve-line lookback, a long table may carry its
/// declaration across arbitrarily many rows. A blank line, heading or
/// non-table prose is a hard boundary; never inherit across sections.
fn table_block_type_for_line(source: &str, line_index: usize) -> Option<u8> {
    let lines: Vec<&str> = source.lines().collect();
    let current = lines.get(line_index)?.trim();
    if !current.contains('|') {
        return None;
    }
    for index in (0..=line_index).rev() {
        let line = lines[index].trim();
        if line.is_empty() || line.starts_with('#') {
            break;
        }
        if !line.contains('|') {
            break;
        }
        if let Some(letter) = super::explicit_type_label(line) {
            return Some(letter);
        }
    }
    None
}

/// Resolve an individual fact against its own original source window.
/// An explicit type in the statement or source line wins only if consistent.
/// A preceding standalone Type heading may own contiguous following lines.
/// Multiple occurrences of the same quote are ambiguous unless they all
/// imply the same explicit identity. Never use the proposed filename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolutionFailure {
    AmbiguousLocator,
    MissingEvidence,
    InvalidLocator,
    NoSourceType,
    ConflictingType,
}

fn resolve_fact_type(source: &str, fact: &CandidateFact) -> Result<u8, ResolutionFailure> {
    use ResolutionFailure as Failure;
    if fact.source_locator_ambiguous {
        return Err(Failure::AmbiguousLocator);
    }
    let quote = fact
        .evidence_quote
        .as_deref()
        .filter(|q| !q.trim().is_empty())
        .ok_or(Failure::MissingEvidence)?;
    let line_index = if let Some(offset) = fact.source_byte_offset {
        let end = offset
            .checked_add(quote.len())
            .ok_or(Failure::InvalidLocator)?;
        if source.get(offset..end) != Some(quote) {
            return Err(Failure::InvalidLocator);
        }
        crate::utils::stringutils::line_index_at_byte_offset(source, offset)
            .ok_or(Failure::InvalidLocator)?
    } else {
        let segments = super::evidence_segments(source);
        if let Some(segment) = segments
            .iter()
            .find(|segment| segment.id == fact.segment_id)
            .filter(|segment| {
                segment.text.trim_end_matches('\n').trim_end_matches('\r')
                    == quote.trim_end_matches('\n').trim_end_matches('\r')
            })
        {
            segment.line_index
        } else {
            let mut matches = source.match_indices(quote);
            let (offset, _) = matches.next().ok_or(Failure::InvalidLocator)?;
            if matches.next().is_some() {
                return Err(Failure::AmbiguousLocator);
            }
            crate::utils::stringutils::line_index_at_byte_offset(source, offset)
                .ok_or(Failure::InvalidLocator)?
        }
    };
    let source_type = super::explicit_type_label(
        source
            .lines()
            .nth(line_index)
            .ok_or(Failure::InvalidLocator)?
            .trim()
            .trim_start_matches('#')
            .trim(),
    )
    .or_else(|| super::section_type_for_line(source, line_index))
    .or_else(|| table_block_type_for_line(source, line_index))
    .ok_or(Failure::NoSourceType)?;
    if let Some(stated) = super::explicit_type_label(&fact.statement)
        && stated != source_type
    {
        return Err(Failure::ConflictingType);
    }
    Ok(source_type)
}

fn source_fact_type(source: &str, fact: &CandidateFact) -> Option<u8> {
    resolve_fact_type(source, fact).ok()
}

fn canonical_type_path(letter: u8) -> String {
    if letter.is_ascii_uppercase() {
        format!(
            "messages/upper_{}.md",
            (letter as char).to_ascii_lowercase()
        )
    } else {
        format!("messages/{}.md", letter as char)
    }
}

/// Explain unresolved P4 facts without changing the acceptance policy.
/// Counts are per occurrence, including repeated coverage candidates.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct ResolutionDiagnostics {
    pub invalid_window: usize,
    pub ambiguous_locator: usize,
    pub missing_evidence: usize,
    pub invalid_offset_or_quote: usize,
    pub no_source_type: usize,
    pub conflicting_type: usize,
}

pub(super) fn resolution_diagnostics(
    candidates: &[MessageCandidate],
    sources: &[&str],
) -> ResolutionDiagnostics {
    let mut result = ResolutionDiagnostics::default();
    for candidate in candidates {
        let source = source_window_index(&candidate.id).and_then(|index| sources.get(index));
        for fact in &candidate.facts {
            let Some(source) = source else {
                result.invalid_window += 1;
                continue;
            };
            match resolve_fact_type(source, fact) {
                Ok(_) => {}
                Err(ResolutionFailure::AmbiguousLocator) => result.ambiguous_locator += 1,
                Err(ResolutionFailure::MissingEvidence) => result.missing_evidence += 1,
                Err(ResolutionFailure::InvalidLocator) => result.invalid_offset_or_quote += 1,
                Err(ResolutionFailure::NoSourceType) => result.no_source_type += 1,
                Err(ResolutionFailure::ConflictingType) => result.conflicting_type += 1,
            }
        }
    }
    result
}

/// Consolidate on a per-fact basis, with original source context. Unknown
/// identities remain in the checkpoint and never enter canonical output.
pub(super) fn resolved_source_facts(
    candidates: &[MessageCandidate],
    sources: &[&str],
) -> (
    std::collections::BTreeMap<String, Vec<(usize, CandidateFact)>>,
    usize,
) {
    let mut groups = std::collections::BTreeMap::new();
    let mut unresolved = 0;
    for candidate in candidates {
        let Some(window) = source_window_index(&candidate.id) else {
            unresolved += candidate.facts.len();
            continue;
        };
        let Some(source) = sources.get(window) else {
            unresolved += candidate.facts.len();
            continue;
        };
        for fact in &candidate.facts {
            let Some(letter) = source_fact_type(source, fact) else {
                unresolved += 1;
                continue;
            };
            groups
                .entry(canonical_type_path(letter))
                .or_insert_with(Vec::new)
                .push((window, fact.clone()));
        }
    }
    (groups, unresolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(statement: &str) -> CandidateFact {
        CandidateFact {
            statement: statement.into(),
            segment_id: "S1".into(),
            evidence_quote: Some("source excerpt".into()),
            source_byte_offset: None,
            source_locator_ambiguous: false,
        }
    }

    #[test]
    fn long_table_inherits_explicit_type_beyond_twelve_rows() {
        let mut source = String::from("Type: A | command\n");
        for _ in 0..20 {
            source.push_str("| parameter | value |\n");
        }
        assert_eq!(table_block_type_for_line(&source, 20), Some(b'A'));
    }

    #[test]
    fn table_type_does_not_cross_section_or_prose_boundaries() {
        assert_eq!(
            table_block_type_for_line("Type: A | command\n\n| other | value |", 2),
            None
        );
        assert_eq!(
            table_block_type_for_line("Type: A | command\nprose\n| other | value |", 2),
            None
        );
        assert_eq!(
            table_block_type_for_line("Type: A | command\n# Other section\n| other | value |", 2),
            None
        );
    }

    #[test]
    fn table_type_uses_nearest_explicit_declaration_preserving_case() {
        let source = "Type: A | command\n| one | value |\nType: a | response\n| two | value |";
        assert_eq!(table_block_type_for_line(source, 1), Some(b'A'));
        assert_eq!(table_block_type_for_line(source, 3), Some(b'a'));
    }

    #[test]
    fn resolves_fact_from_long_table_with_exact_offset() {
        let mut source = String::from("Type: A | command\n");
        for _ in 0..20 {
            source.push_str("| parameter | value |\n");
        }
        let quote = "| final | value |";
        let offset = source.len();
        source.push_str(quote);
        let candidate = MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "messages/incorrect.md".into(),
            facts: vec![CandidateFact {
                statement: "final parameter".into(),
                segment_id: "S22".into(),
                evidence_quote: Some(quote.into()),
                source_byte_offset: Some(offset),
                source_locator_ambiguous: false,
            }],
        };
        let (groups, unresolved) = resolved_source_facts(&[candidate], &[&source]);
        assert_eq!(unresolved, 0);
        assert_eq!(groups["messages/upper_a.md"].len(), 1);
    }

    #[test]
    fn diagnostics_and_projection_agree_on_ambiguous_legacy_quote() {
        let source = "Type: A\nrepeat\nrepeat\n";
        let candidate = MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "messages/upper_a.md".into(),
            facts: vec![CandidateFact {
                statement: "parameter".into(),
                segment_id: "missing".into(),
                evidence_quote: Some("repeat".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            }],
        };
        let (groups, unresolved) = resolved_source_facts(&[candidate.clone()], &[source]);
        assert!(groups.is_empty());
        assert_eq!(unresolved, 1);
        let diagnostics = resolution_diagnostics(&[candidate], &[source]);
        assert_eq!(diagnostics.ambiguous_locator, 1);
    }

    #[test]
    fn diagnoses_untyped_fact_without_promoting_it() {
        let source = "| parameter | value |";
        let candidate = MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "messages/upper_a.md".into(),
            facts: vec![CandidateFact {
                statement: "parameter".into(),
                segment_id: "S1".into(),
                evidence_quote: Some(source.into()),
                source_byte_offset: Some(0),
                source_locator_ambiguous: false,
            }],
        };
        let diagnostics = resolution_diagnostics(&[candidate], &[source]);
        assert_eq!(diagnostics.no_source_type, 1);
        assert_eq!(diagnostics.conflicting_type, 0);
    }

    #[test]
    fn counts_duplicate_source_facts_without_merging_candidate_identities() {
        let candidates = collect_candidates(
            3,
            [
                ("messages/a.md".into(), vec![fact("shared"), fact("unique")]),
                ("messages/upper_a.md".into(), vec![fact("shared")]),
            ],
        );
        assert_eq!(distinct_source_fact_count(&candidates), 2);
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn consolidates_explicit_source_type_across_different_proposed_names() {
        let candidates = vec![
            MessageCandidate {
                id: "window-2-candidate-0".into(),
                proposed_name: "messages/status.md".into(),
                facts: vec![CandidateFact {
                    statement: "first".into(),
                    segment_id: "S1".into(),
                    evidence_quote: Some("Type: A | modem status".into()),
                    source_byte_offset: None,
                    source_locator_ambiguous: false,
                }],
            },
            MessageCandidate {
                id: "coverage-window-3-candidate-0".into(),
                proposed_name: "messages/another_alias.md".into(),
                facts: vec![CandidateFact {
                    statement: "second".into(),
                    segment_id: "S1".into(),
                    evidence_quote: Some("Type: A | modem status".into()),
                    source_byte_offset: None,
                    source_locator_ambiguous: false,
                }],
            },
        ];
        let (groups, unresolved) = resolved_source_facts(
            &candidates,
            &[
                "Type: A | modem status",
                "Type: A | modem status",
                "Type: A | modem status",
                "Type: A | modem status",
            ],
        );
        assert_eq!(unresolved, 0);
        assert_eq!(groups["messages/upper_a.md"].len(), 2);
        assert_eq!(groups["messages/upper_a.md"][0].0, 2);
        assert_eq!(groups["messages/upper_a.md"][1].0, 3);
    }

    #[test]
    fn never_merges_upper_and_lower_or_untyped_candidates() {
        let candidates = vec![
            MessageCandidate {
                id: "window-0-candidate-0".into(),
                proposed_name: "messages/a.md".into(),
                facts: vec![CandidateFact {
                    statement: "upper".into(),
                    segment_id: "S1".into(),
                    evidence_quote: Some("Type: A | command".into()),
                    source_byte_offset: None,
                    source_locator_ambiguous: false,
                }],
            },
            MessageCandidate {
                id: "window-0-candidate-1".into(),
                proposed_name: "messages/a.md".into(),
                facts: vec![CandidateFact {
                    statement: "lower".into(),
                    segment_id: "S2".into(),
                    evidence_quote: Some("Type: a | report".into()),
                    source_byte_offset: None,
                    source_locator_ambiguous: false,
                }],
            },
            MessageCandidate {
                id: "window-0-candidate-2".into(),
                proposed_name: "messages/a.md".into(),
                facts: vec![fact("unknown")],
            },
        ];
        let (groups, unresolved) = resolved_source_facts(
            &candidates,
            &["Type: A | command\nType: a | report\n\nsource excerpt"],
        );
        assert_eq!(groups["messages/upper_a.md"].len(), 1);
        assert_eq!(groups["messages/a.md"].len(), 1);
        assert_eq!(unresolved, 1);
    }

    #[test]
    fn source_section_resolves_untyped_fact_independently_of_candidate_name() {
        let source = "Type: A\nfield one is required\nfield two is optional\n\nType: a\nfield three is required";
        let candidates = vec![MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "messages/unknown.md".into(),
            facts: vec![
                CandidateFact {
                    statement: "first".into(),
                    segment_id: "S2".into(),
                    evidence_quote: Some("field one is required".into()),
                    source_byte_offset: None,
                    source_locator_ambiguous: false,
                },
                CandidateFact {
                    statement: "second".into(),
                    segment_id: "S5".into(),
                    evidence_quote: Some("field three is required".into()),
                    source_byte_offset: None,
                    source_locator_ambiguous: false,
                },
            ],
        }];
        let (groups, unresolved) = resolved_source_facts(&candidates, &[source]);
        assert_eq!(unresolved, 0);
        assert_eq!(groups["messages/upper_a.md"].len(), 1);
        assert_eq!(groups["messages/a.md"].len(), 1);
    }

    #[test]
    fn recovery_window_stores_parent_byte_offset() {
        let parent = "Präambel 🛰️\nType: A\nshared field\n\nType: a\nshared field\n";
        let window = "Type: a\nshared field\n";
        let response = r#"{"files":[{"path":"messages/anything.md","facts":[{"statement":"lower field","segment_id":"S2"}]}]}"#;
        let candidates = parse_candidates_in_parent(response, window, parent, 0, 0);
        assert_eq!(candidates.len(), 1);
        let fact = &candidates[0].facts[0];
        let expected = parent.rfind("shared field").unwrap();
        assert_eq!(fact.source_byte_offset, Some(expected));
        assert!(!fact.source_locator_ambiguous);
        let (groups, unresolved) = resolved_source_facts(&candidates, &[parent]);
        assert_eq!(unresolved, 0);
        assert_eq!(groups["messages/a.md"].len(), 1);
    }

    #[test]
    fn repeated_recovery_window_never_guesses_parent_location() {
        let window = "Type: A\nshared field\n";
        let parent = format!("{window}middle\n{window}");
        let response = r#"{"files":[{"path":"messages/anything.md","facts":[{"statement":"shared field","segment_id":"S2"}]}]}"#;
        let candidates = parse_candidates_in_parent(response, window, &parent, 0, 0);
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].facts[0].source_locator_ambiguous);
        let (groups, unresolved) = resolved_source_facts(&candidates, &[&parent]);
        assert!(groups.is_empty());
        assert_eq!(unresolved, 1);
    }

    #[test]
    fn segment_locator_disambiguates_repeated_source_text() {
        let source = "Type: A\nshared field\n\nType: a\nshared field";
        let candidates = vec![MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "messages/alias.md".into(),
            facts: vec![
                CandidateFact {
                    statement: "upper shared field".into(),
                    segment_id: "S2".into(),
                    evidence_quote: Some("shared field".into()),
                    source_byte_offset: None,
                    source_locator_ambiguous: false,
                },
                CandidateFact {
                    statement: "lower shared field".into(),
                    segment_id: "S4".into(),
                    evidence_quote: Some("shared field".into()),
                    source_byte_offset: None,
                    source_locator_ambiguous: false,
                },
            ],
        }];
        let (groups, unresolved) = resolved_source_facts(&candidates, &[source]);
        assert_eq!(unresolved, 0);
        assert_eq!(groups["messages/upper_a.md"].len(), 1);
        assert_eq!(groups["messages/a.md"].len(), 1);
    }

    #[test]
    fn conflicting_repeated_quote_stays_unresolved() {
        let source = "Type: A\nshared field\n\nType: a\nshared field";
        let candidates = vec![MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "messages/a.md".into(),
            facts: vec![CandidateFact {
                statement: "shared field".into(),
                segment_id: "S99".into(),
                evidence_quote: Some("shared field".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            }],
        }];
        let (groups, unresolved) = resolved_source_facts(&candidates, &[source]);
        assert!(groups.is_empty());
        assert_eq!(unresolved, 1);
    }

    #[test]
    fn same_label_does_not_collapse_distinct_candidates() {
        let candidates = collect_candidates(
            3,
            [
                ("a".into(), vec![fact("antenna request")]),
                ("a".into(), vec![fact("modem interval")]),
            ],
        );
        assert_eq!(candidates.len(), 2);
        assert_ne!(candidates[0].id, candidates[1].id);
        assert_eq!(candidates[0].proposed_name, candidates[1].proposed_name);
    }

    #[test]
    fn arbitrary_free_text_names_survive_collection() {
        let label = "Get extant status / antenna state (legacy name)";
        let candidates = collect_candidates(8, [(label.into(), vec![fact("status")])]);
        assert_eq!(candidates[0].proposed_name, label);
        assert_eq!(candidates[0].facts[0].segment_id, "S1");
    }

    #[test]
    fn case_distinct_types_are_not_merged_during_collection() {
        let candidates = collect_candidates(
            2,
            [
                ("A".into(), vec![fact("modem status interval")]),
                ("a".into(), vec![fact("antenna L interval")]),
            ],
        );
        assert_eq!(candidates.len(), 2);
        assert_ne!(candidates[0].proposed_name, candidates[1].proposed_name);
    }

    #[test]
    fn candidate_roundtrip_preserves_original_names_and_evidence() {
        let candidates =
            collect_candidates(5, [("Antenna Status".into(), vec![fact("status report")])]);
        let json = serde_json::to_string(&candidates).unwrap();
        let restored: Vec<MessageCandidate> = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, candidates);
    }

    #[test]
    fn stages_unknown_descriptive_identity_with_source_evidence() {
        let source = "Antenna status reports the active satellite.";
        let response = r#"{"files":[{"path":"messages/antenna_status.md","facts":[{"statement":"Reports the active satellite","segment_id":"S1"}]}]}"#;
        let candidates = parse_candidates(response, source, 7, 0);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].proposed_name, "messages/antenna_status.md");
        assert_eq!(
            candidates[0].facts[0].evidence_quote.as_deref(),
            Some(source)
        );
    }

    #[test]
    fn staging_rejects_fabricated_locators_and_type_contradictions() {
        let source = "Type: A | Modem alive";
        let response = r#"{"files":[{"path":"messages/custom.md","facts":[{"statement":"Type: B | Beam","segment_id":"S1"},{"statement":"Valid fact","segment_id":"S999"},{"statement":"Type: A | Modem alive","segment_id":"S1"}]}]}"#;
        let candidates = parse_candidates(response, source, 1, 0);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].facts.len(), 1);
        assert_eq!(candidates[0].facts[0].statement, "Type: A | Modem alive");
    }

    #[test]
    fn classification_never_trusts_proposed_filename() {
        let candidate = MessageCandidate {
            id: "one".into(),
            proposed_name: "messages/a.md".into(),
            facts: vec![CandidateFact {
                statement: "Antenna message".into(),
                segment_id: "S1".into(),
                evidence_quote: Some("Type: B | Source declaration".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            }],
        };
        assert_eq!(
            classify_candidate(&candidate),
            CandidateIdentity::Resolved("messages/upper_b.md".into())
        );
    }

    #[test]
    fn classification_keeps_unknown_and_conflicting_types_unresolved() {
        let mut candidate = MessageCandidate {
            id: "one".into(),
            proposed_name: "messages/a.md".into(),
            facts: vec![fact("no source type")],
        };
        assert_eq!(
            classify_candidate(&candidate),
            CandidateIdentity::Unresolved
        );
        candidate.facts[0].evidence_quote = Some("Type: A | one".into());
        candidate.facts.push(CandidateFact {
            statement: "two".into(),
            segment_id: "S2".into(),
            evidence_quote: Some("Type: a | two".into()),
            source_byte_offset: None,
            source_locator_ambiguous: false,
        });
        assert_eq!(
            classify_candidate(&candidate),
            CandidateIdentity::Unresolved
        );
    }

    #[test]
    fn malformed_json_is_not_staged() {
        let candidates = parse_candidates(r#"{"files":["#, "source", 0, 0);
        assert!(candidates.is_empty());
    }

    #[test]
    fn identity_decisions_are_explicit_and_auditable() {
        let decision = IdentityDecision::Unresolved {
            left: "window-1-candidate-0".into(),
            right: "window-2-candidate-0".into(),
            reason: "same label without identity evidence".into(),
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("unresolved"));
        assert!(json.contains("reason"));
        let merge = IdentityDecision::Merge {
            left: "a".into(),
            right: "b".into(),
            reason: "explicit source equivalence".into(),
        };
        let split = IdentityDecision::Split {
            left: "A".into(),
            right: "a".into(),
            reason: "different source-declared directions".into(),
        };
        assert!(serde_json::to_string(&merge).unwrap().contains("merge"));
        assert!(serde_json::to_string(&split).unwrap().contains("split"));
    }
}
