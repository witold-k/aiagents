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

// A table caption ends a flattened table irrespective of the document's
// numbering scheme (e.g. "Table A-2.", "Table 17.", "Table 2-2.").
fn is_table_caption(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("Table ") else {
        return false;
    };
    let identifier = rest.split_whitespace().next().unwrap_or("");
    identifier
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric())
        && identifier
            .chars()
            .any(|ch| ch.is_ascii_digit() || ch == '-')
}

/// PDF-to-Markdown conversion sometimes flattens a message table into
/// consecutive prose lines. Within the message-type table only, a row's
/// explicit `Type: X |` cell owns following continuation lines until the next
/// declaration, heading, table boundary or a bounded distance. Blank lines
/// alone are not sufficient boundaries in these converted tables.
fn flattened_message_table_type(source: &str, line_index: usize) -> Option<u8> {
    let lines: Vec<&str> = source.lines().collect();
    let mut in_message_table = false;
    for line in lines.iter().take(line_index + 1) {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            // Structural context, not a specification-specific section number.
            // A new heading always ends the preceding table's ownership.
            in_message_table = trimmed
                .trim_start_matches('#')
                .trim()
                .to_ascii_lowercase()
                .contains("message types");
        }
    }
    if !in_message_table {
        return None;
    }
    for distance in 0..=12 {
        let index = line_index.checked_sub(distance)?;
        let line = lines.get(index)?.trim();
        if line.starts_with('#')
            || line.starts_with("```")
            || is_table_caption(line)
            || line.starts_with("| Type")
            || line.starts_with("| ---")
        {
            break;
        }
        if let Some(letter) = super::explicit_type_label(line) {
            // A row declaration is not an open-ended standalone heading.
            return line.contains('|').then_some(letter);
        }
        // An unrecognized or damaged declaration still ends the previous
        // row's ownership; never guess a replacement for its type.
        if line.starts_with("Type:") || line.starts_with("| Type:") {
            break;
        }
    }
    None
}

/// A single-letter command is evidence only inside a fenced example under
/// an explicit message-example section. Neither prose nor arbitrary code
/// blocks establish message identity.
fn message_example_type_for_line(document: &str, line_index: usize) -> Option<u8> {
    let mut heading_is_messages = false;
    let mut in_fence = false;
    let mut fence_is_example = false;
    for (index, raw) in document.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with('#') && !in_fence {
            let heading = line.trim_start_matches('#').trim().to_ascii_lowercase();
            heading_is_messages = heading.contains("messages from");
        }
        if line.starts_with("```") {
            if in_fence {
                in_fence = false;
                fence_is_example = false;
            } else {
                in_fence = true;
                fence_is_example = heading_is_messages
                    && matches!(line.strip_prefix("```").unwrap_or(""), "txt" | "text" | "");
            }
            if index == line_index {
                return None;
            }
            continue;
        }
        if index == line_index {
            if !in_fence || !fence_is_example {
                return None;
            }
            let bytes = line.as_bytes();
            if bytes.len() < 3
                || !bytes[0].is_ascii_alphabetic()
                || !bytes[1].is_ascii_whitespace()
                || !bytes[2..].contains(&b'#')
            {
                return None;
            }
            return Some(bytes[0]);
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
    if matches!(
        local,
        Err(ResolutionFailure::AmbiguousLocator
            | ResolutionFailure::MissingEvidence
            | ResolutionFailure::InvalidLocator
            | ResolutionFailure::ConflictingType)
    ) {
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
        flattened_message_table_type(document, line),
        message_example_type_for_line(document, line),
    ]
    .into_iter()
    .flatten()
    {
        if declared.is_some_and(|old| old != signal) {
            return Err(ResolutionFailure::ConflictingType);
        }
        declared = Some(signal);
    }
    if let (Ok(local_letter), Some(document_letter)) = (&local, declared)
        && *local_letter != document_letter
    {
        return Err(ResolutionFailure::ConflictingType);
    }
    let letter = declared
        .or_else(|| local.ok())
        .ok_or(ResolutionFailure::NoSourceType)?;
    if super::explicit_type_label(&fact.statement).is_some_and(|stated| stated != letter) {
        return Err(ResolutionFailure::ConflictingType);
    }
    Ok(letter)
}

#[cfg(test)]
mod example_identity_tests {
    use super::message_example_type_for_line;

    #[test]
    fn recognizes_only_message_examples_and_preserves_case() {
        let source = "## Messages from Modem to Antenna\n```txt\nB 100 200 # frequency\na 60 # alive\n```\n## Other examples\n```txt\nZ 100\n```";
        assert_eq!(message_example_type_for_line(source, 2), Some(b'B'));
        assert_eq!(message_example_type_for_line(source, 3), Some(b'a'));
        assert_eq!(message_example_type_for_line(source, 7), None);
    }

    #[test]
    fn rejects_prose_continuations_and_unfenced_commands() {
        let source = "## Messages from Controller\nA 10\n```txt\nA 10 # ping\ncontinued line\n# comment\n```";
        assert_eq!(message_example_type_for_line(source, 1), None);
        assert_eq!(message_example_type_for_line(source, 3), Some(b'A'));
        assert_eq!(message_example_type_for_line(source, 4), None);
        assert_eq!(message_example_type_for_line(source, 5), None);
    }
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
    // Sample across the whole document instead of showing only the first
    // unresolved rows (which tend to cluster in the same early table).
    facts.dedup_by_key(|(absolute, _)| *absolute);
    let count = facts.len();
    if limit == 0 {
        return Vec::new();
    }
    let stride = count.div_ceil(limit);
    let mut samples = Vec::new();
    for (absolute, statement) in facts.into_iter().step_by(stride.max(1)) {
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
        let lines: Vec<&str> = document.lines().collect();
        let from = line_number.saturating_sub(4);
        let to = (line_number + 2).min(lines.len());
        let nearby = (from..to)
            .map(|i| {
                let content: String = lines[i].chars().take(140).collect();
                format!("{}:{content}", i + 1)
            })
            .collect::<Vec<_>>()
            .join(" || ");
        samples.push(format!(
            "line={line_number} last_type=[{declaration}] last_heading=[{heading}] statement={excerpt:?} nearby=[{nearby}]"
        ));
        if samples.len() >= limit {
            break;
        }
    }
    samples
}

/// Read-only triage of facts with no proven message identity. These are
/// structural hints, not assertions of semantic ownership or irrelevance.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct UnresolvedTriage {
    pub example_line: usize,
    pub parameter_table_row: usize,
    pub other: usize,
}

fn unresolved_source_shape(document: &str, line_index: usize) -> &'static str {
    let lines: Vec<&str> = document.lines().collect();
    let mut fenced = false;
    for line in lines.iter().take(line_index + 1) {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
    }
    let line = lines.get(line_index).copied().unwrap_or("").trim();
    if fenced
        && line
            .as_bytes()
            .first()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && line
            .as_bytes()
            .get(1)
            .is_some_and(|c| c.is_ascii_whitespace())
    {
        return "example";
    }
    if line.starts_with('|') {
        // Look for a column header in this uninterrupted Markdown table.
        for prior in lines[..line_index].iter().rev() {
            let prior = prior.trim();
            if prior.is_empty() || prior.starts_with('#') || !prior.starts_with('|') {
                break;
            }
            let lower = prior.to_ascii_lowercase();
            if lower.contains("parameter") && lower.contains("description") {
                return "parameter";
            }
        }
    }
    "other"
}

pub(super) fn unresolved_triage_in_document(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    document: &str,
) -> UnresolvedTriage {
    let mut result = UnresolvedTriage::default();
    for candidate in candidates {
        let Some((window, start)) = source_window_index(&candidate.id)
            .and_then(|index| sources.get(index).zip(starts.get(index)))
        else {
            continue;
        };
        for fact in &candidate.facts {
            if !matches!(
                resolve_fact_type_in_document(window, *start, document, fact),
                Err(ResolutionFailure::NoSourceType)
            ) {
                continue;
            }
            let Some(absolute) = fact
                .source_byte_offset
                .and_then(|offset| start.checked_add(offset))
            else {
                result.other += 1;
                continue;
            };
            let Some(line) =
                crate::utils::stringutils::line_index_at_byte_offset(document, absolute)
            else {
                result.other += 1;
                continue;
            };
            match unresolved_source_shape(document, line) {
                "example" => result.example_line += 1,
                "parameter" => result.parameter_table_row += 1,
                _ => result.other += 1,
            }
        }
    }
    result
}

/// Independent, source-backed parameter assertions. They are deliberately
/// not assigned a message identity and are not yet publication-approved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ParameterFact {
    pub statement: String,
    pub parameter: String,
    pub section: String,
    pub evidence_quote: String,
    pub source_byte_offset: usize,
}

/// Inventory only: a Markdown parameter table header and an exact source
/// locator are required. This does not reinterpret prose or damaged tables.
pub(super) fn parameter_facts_in_document(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    document: &str,
) -> Vec<ParameterFact> {
    let lines: Vec<&str> = document.lines().collect();
    let mut inventory = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for candidate in candidates {
        let Some((window, start)) = source_window_index(&candidate.id)
            .and_then(|index| sources.get(index).zip(starts.get(index)))
        else {
            continue;
        };
        for fact in &candidate.facts {
            if !matches!(
                resolve_fact_type_in_document(window, *start, document, fact),
                Err(ResolutionFailure::NoSourceType)
            ) {
                continue;
            }
            let Some(absolute) = fact
                .source_byte_offset
                .and_then(|offset| start.checked_add(offset))
            else {
                continue;
            };
            let Some(quote) = fact.evidence_quote.as_deref() else {
                continue;
            };
            if document.get(absolute..absolute.saturating_add(quote.len())) != Some(quote) {
                continue;
            }
            let Some(line_index) =
                crate::utils::stringutils::line_index_at_byte_offset(document, absolute)
            else {
                continue;
            };
            if unresolved_source_shape(document, line_index) != "parameter" {
                continue;
            }
            let Some(row) = lines.get(line_index) else {
                continue;
            };
            let Some(parameter) = row.trim().trim_matches('|').split('|').next() else {
                continue;
            };
            let parameter = parameter.trim();
            if parameter.is_empty()
                || !parameter
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                continue;
            }
            let section = lines[..line_index]
                .iter()
                .rev()
                .find(|line| line.trim_start().starts_with('#'))
                .map(|line| line.trim().to_owned())
                .unwrap_or_default();
            if seen.insert((absolute, fact.statement.clone(), quote.to_owned())) {
                inventory.push(ParameterFact {
                    statement: fact.statement.clone(),
                    parameter: parameter.to_owned(),
                    section,
                    evidence_quote: quote.to_owned(),
                    source_byte_offset: absolute,
                });
            }
        }
    }
    inventory.sort_by_key(|fact| fact.source_byte_offset);
    inventory
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
mod parameter_inventory_tests {
    use super::{CandidateFact, MessageCandidate, parameter_facts_in_document};

    #[test]
    fn preserves_independent_parameters_and_deduplicates_overlapping_candidates() {
        let source = "# Appendix\n## Hardware\n| Parameter | Description | Data Type |\n| --- | --- | --- |\n| antAz | Antenna azimuth | degrees |\n";
        let quote = "| antAz | Antenna azimuth | degrees |";
        let offset = source.find(quote).unwrap();
        let fact = CandidateFact {
            statement: "Antenna azimuth".into(),
            segment_id: "test".into(),
            evidence_quote: Some(quote.into()),
            source_byte_offset: Some(offset),
            source_locator_ambiguous: false,
        };
        let candidates = vec![
            MessageCandidate {
                id: "window-0-candidate-0".into(),
                proposed_name: "messages/s.md".into(),
                facts: vec![fact.clone()],
            },
            MessageCandidate {
                id: "window-0-candidate-1".into(),
                proposed_name: "anything".into(),
                facts: vec![fact],
            },
        ];
        let inventory = parameter_facts_in_document(&candidates, &[source], &[0], source);
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].parameter, "antAz");
        assert_eq!(inventory[0].section, "## Hardware");
        assert_eq!(inventory[0].source_byte_offset, offset);
        assert_eq!(inventory[0].evidence_quote, quote);
    }
}

#[cfg(test)]
#[path = "../../tests/workflows/message_identity_test.rs"]
mod tests;
