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
    // Independent ownership signals must agree. A nearby heading cannot
    // silently override a contradictory declaration in a table.
    let line = source
        .lines()
        .nth(line_index)
        .ok_or(Failure::InvalidLocator)?
        .trim()
        .trim_start_matches('#')
        .trim();
    let signals = [
        super::explicit_type_label(line),
        super::section_type_for_line(source, line_index),
        table_block_type_for_line(source, line_index),
    ];
    let mut declared = None;
    for signal in signals.into_iter().flatten() {
        if declared.is_some_and(|previous| previous != signal) {
            return Err(Failure::ConflictingType);
        }
        declared = Some(signal);
    }
    let source_type = declared.ok_or(Failure::NoSourceType)?;
    if let Some(stated) = super::explicit_type_label(&fact.statement)
        && stated != source_type
    {
        return Err(Failure::ConflictingType);
    }
    Ok(source_type)
}

// Resolve against the complete normalized document, using the verified
// chunk-relative byte offset. This preserves ownership across chunk boundaries
// without trusting the model's proposed filename or its remembered context.
fn resolve_fact_type_in_document(
    window: &str,
    window_start: usize,
    document: &str,
    fact: &CandidateFact,
) -> Result<u8, ResolutionFailure> {
    let local = resolve_fact_type(window, fact);
    if !matches!(local, Err(ResolutionFailure::NoSourceType)) {
        return local;
    }
    let offset = fact
        .source_byte_offset
        .ok_or(ResolutionFailure::InvalidLocator)?;
    let quote = fact
        .evidence_quote
        .as_deref()
        .ok_or(ResolutionFailure::MissingEvidence)?;
    let absolute = window_start
        .checked_add(offset)
        .ok_or(ResolutionFailure::InvalidLocator)?;
    let end = absolute
        .checked_add(quote.len())
        .ok_or(ResolutionFailure::InvalidLocator)?;
    if document.get(absolute..end) != Some(quote) {
        return Err(ResolutionFailure::InvalidLocator);
    }
    let line = crate::utils::stringutils::line_index_at_byte_offset(document, absolute)
        .ok_or(ResolutionFailure::InvalidLocator)?;
    let source_line = document
        .lines()
        .nth(line)
        .ok_or(ResolutionFailure::InvalidLocator)?;
    let mut declared = None;
    for signal in [
        super::explicit_type_label(source_line),
        super::section_type_for_line(document, line),
        table_block_type_for_line(document, line),
    ]
    .into_iter()
    .flatten()
    {
        if declared.is_some_and(|old| old != signal) {
            return Err(ResolutionFailure::ConflictingType);
        }
        declared = Some(signal);
    }
    let letter = declared.ok_or(ResolutionFailure::NoSourceType)?;
    if super::explicit_type_label(&fact.statement).is_some_and(|stated| stated != letter) {
        return Err(ResolutionFailure::ConflictingType);
    }
    Ok(letter)
}

/// A source-ordered history probe. This is deliberately diagnostic: a previous
/// type declaration is context, not automatically proof of ownership. The
/// replay can expose where the current strict resolver loses that context.
pub(super) fn unresolved_history_samples(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    document: &str,
    limit: usize,
) -> Vec<String> {
    let mut facts = Vec::new();
    for candidate in candidates {
        let Some(index) = source_window_index(&candidate.id) else {
            continue;
        };
        let Some((window, start)) = sources.get(index).zip(starts.get(index)) else {
            continue;
        };
        for fact in &candidate.facts {
            if !matches!(
                resolve_fact_type_in_document(window, *start, document, fact),
                Err(ResolutionFailure::NoSourceType)
            ) {
                continue;
            }
            let Some(offset) = fact.source_byte_offset else {
                continue;
            };
            let Some(absolute) = start.checked_add(offset) else {
                continue;
            };
            facts.push((absolute, fact.statement.as_str()));
        }
    }
    facts.sort_unstable();
    let mut samples = Vec::new();
    let mut last_offset = None;
    for (absolute, statement) in facts {
        if last_offset == Some(absolute) {
            continue;
        }
        last_offset = Some(absolute);
        let Some(prefix) = document.get(..absolute) else {
            continue;
        };
        let line_number = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let mut last_declaration = None;
        let mut last_heading = None;
        for (index, line) in prefix.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                last_heading = Some((index + 1, trimmed));
            }
            // Only a standalone declaration is eligible for a history hint.
            // Inline table references and examples are not active headings.
            let heading = trimmed.trim_start_matches('#').trim();
            if heading.starts_with("Type:")
                && super::explicit_type_label(heading).is_some()
                && heading
                    .strip_prefix("Type:")
                    .is_some_and(|rest| rest.trim().len() == 1)
            {
                last_declaration = Some((index + 1, heading));
            }
        }
        let declaration = last_declaration
            .map(|(line, value)| format!("line {line}: {value}"))
            .unwrap_or_else(|| "none".into());
        let heading = last_heading
            .map(|(line, value)| format!("line {line}: {value}"))
            .unwrap_or_else(|| "none".into());
        let excerpt: String = statement.chars().take(100).collect();
        samples.push(format!(
            "line={line_number} last_type=[{declaration}] last_heading=[{heading}] statement={excerpt:?}"
        ));
        if samples.len() >= limit {
            break;
        }
    }
    samples
}

pub(super) fn resolution_diagnostics_in_document(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    document: &str,
) -> ResolutionDiagnostics {
    let mut result = ResolutionDiagnostics::default();
    for candidate in candidates {
        let window = source_window_index(&candidate.id);
        for fact in &candidate.facts {
            let Some((source, start)) = window.and_then(|i| sources.get(i).zip(starts.get(i)))
            else {
                result.invalid_window += 1;
                continue;
            };
            match resolve_fact_type_in_document(source, *start, document, fact) {
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

pub(super) fn resolved_source_facts_in_document(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    document: &str,
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
        let Some((source, start)) = sources.get(window).zip(starts.get(window)) else {
            unresolved += candidate.facts.len();
            continue;
        };
        for fact in &candidate.facts {
            match resolve_fact_type_in_document(source, *start, document, fact) {
                Ok(letter) => groups
                    .entry(canonical_type_path(letter))
                    .or_insert_with(Vec::new)
                    .push((window, fact.clone())),
                Err(_) => unresolved += 1,
            }
        }
    }
    (groups, unresolved)
}

#[cfg(test)]
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

#[cfg(test)]
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
#[cfg(test)]
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
#[path = "../../tests/workflows/message_identity_test.rs"]
mod tests;
