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
/// declaration, heading or table boundary. Blank lines alone are not
/// sufficient boundaries in these converted tables.
#[cfg(test)]
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
    // Follow the table's structural boundaries, not an arbitrary number
    // of lines. Converted table cells can span many physical lines.
    // Stop at any new declaration (including malformed ones), heading,
    // caption, fence or table header. Never infer identity from filenames.
    for index in (0..=line_index).rev() {
        let line = lines[index].trim();
        if line.starts_with('#')
            || line.starts_with("```")
            || is_table_caption(line)
            || line.starts_with("| Type")
            || line.starts_with("| ---")
        {
            break;
        }
        if let Some(letter) = super::explicit_type_label(line) {
            return line.contains('|').then_some(letter);
        }
        if line.starts_with("Type:") || line.starts_with("| Type:") {
            break;
        }
    }
    None
}

#[cfg(test)]
mod structural_table_ownership_tests {
    use super::flattened_message_table_type;

    #[test]
    fn long_converted_table_row_keeps_its_explicit_identity() {
        let mut lines = vec![
            "## Message Types".to_owned(),
            "Type: q | status | 3 | int value".to_owned(),
        ];
        lines.extend((0..25).map(|n| format!("Continuation of parameter {n}")));
        let document = lines.join("\n");
        assert_eq!(flattened_message_table_type(&document, 26), Some(b'q'));
    }

    #[test]
    fn new_row_and_heading_are_hard_boundaries() {
        let source = "## Message Types\nType: Q | description\ncontinuation\nType: q | response\ncontinuation\n## Appendix\nother prose";
        assert_eq!(flattened_message_table_type(source, 2), Some(b'Q'));
        assert_eq!(flattened_message_table_type(source, 4), Some(b'q'));
        assert_eq!(flattened_message_table_type(source, 6), None);
    }

    #[test]
    fn damaged_declaration_does_not_inherit_previous_row() {
        let source = "## Message Types\nType: Q | description\nType: ?? | damaged\ncontinuation";
        assert_eq!(flattened_message_table_type(source, 3), None);
    }
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

/// Structural nodes carry source positions, not inferred protocol identities.
/// A table row may be perfectly located while its message owner remains unknown.
#[derive(Debug)]
#[allow(dead_code)] // Structural metadata is retained for later source-backed consumers.
struct SourceSection {
    heading_line: usize,
    level: usize,
    parent: Option<usize>,
}

#[derive(Debug)]
#[allow(dead_code)] // Structural metadata is retained for later source-backed consumers.
struct SourceTable {
    section: Option<usize>,
    rows: Vec<usize>,
}

#[derive(Debug)]
#[allow(dead_code)] // Structural metadata is retained for later source-backed consumers.
struct SourceRow {
    table: usize,
    line: usize,
    cells: Vec<String>,
}

#[derive(Debug)]
#[allow(dead_code)] // Structural metadata is retained for later source-backed consumers.
struct SourceTree {
    sections: Vec<SourceSection>,
    tables: Vec<SourceTable>,
    rows: Vec<SourceRow>,
    /// Each source line's innermost enclosing heading.
    section_at_line: Vec<Option<usize>>,
    /// A physical row can be looked up without scanning other tables.
    row_at_line: Vec<Option<usize>>,
}

impl SourceTree {
    fn from_document(document: &str) -> Self {
        let mut tree = Self {
            sections: Vec::new(),
            tables: Vec::new(),
            rows: Vec::new(),
            section_at_line: Vec::new(),
            row_at_line: Vec::new(),
        };
        let mut heading_stack: Vec<usize> = Vec::new();
        let mut active_table: Option<usize> = None;
        let mut in_fence = false;
        for (line_index, raw) in document.lines().enumerate() {
            let line = raw.trim();
            if line.starts_with("```") {
                in_fence = !in_fence;
                active_table = None;
                tree.section_at_line.push(heading_stack.last().copied());
                tree.row_at_line.push(None);
                continue;
            }
            if !in_fence {
                let level = line.bytes().take_while(|b| *b == b'#').count();
                if (1..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
                    while heading_stack
                        .last()
                        .is_some_and(|index| tree.sections[*index].level >= level)
                    {
                        heading_stack.pop();
                    }
                    let parent = heading_stack.last().copied();
                    let index = tree.sections.len();
                    tree.sections.push(SourceSection {
                        heading_line: line_index,
                        level,
                        parent,
                    });
                    heading_stack.push(index);
                    active_table = None;
                }
            }
            let section = heading_stack.last().copied();
            tree.section_at_line.push(section);
            let row_cells = if in_fence || line.starts_with('#') {
                None
            } else if line.starts_with('|') && line.ends_with('|') {
                let cells: Vec<String> = line
                    .trim_matches('|')
                    .split('|')
                    .map(|cell| cell.trim().to_owned())
                    .collect();
                (cells.len() >= 2).then_some(cells)
            } else if line.starts_with("Type:") && line.contains('|') {
                Some(line.split('|').map(|cell| cell.trim().to_owned()).collect())
            } else {
                None
            };
            if let Some(cells) = row_cells {
                let table = *active_table.get_or_insert_with(|| {
                    let index = tree.tables.len();
                    tree.tables.push(SourceTable {
                        section,
                        rows: Vec::new(),
                    });
                    index
                });
                let index = tree.rows.len();
                tree.rows.push(SourceRow {
                    table,
                    line: line_index,
                    cells,
                });
                tree.tables[table].rows.push(index);
                tree.row_at_line.push(Some(index));
            } else {
                // A blank line may separate Markdown table blocks. Flattened
                // conversions require later explicit rows to start new blocks.
                if !line.is_empty() {
                    active_table = None;
                }
                tree.row_at_line.push(None);
            }
        }
        tree
    }

    /// A flattened table may have lost its pipe delimiters on continuation
    /// lines. Require a real table header and an explicit one-letter Type row;
    /// never infer ownership from prose or from a section heading alone.
    fn flattened_table_owner(&self, document: &str, line_index: usize) -> Option<u8> {
        let lines: Vec<&str> = document.lines().collect();
        let mut header_seen = false;
        let mut owner = None;
        let mut in_fence = false;
        for (index, raw) in lines.iter().enumerate().take(line_index + 1) {
            let line = raw.trim();
            if line.starts_with("```") {
                in_fence = !in_fence;
                header_seen = false;
                owner = None;
            }
            if in_fence {
                continue;
            }
            if line.starts_with('#') || is_table_caption(line) {
                header_seen = false;
                owner = None;
            } else if line.starts_with("Type | Description |")
                || line.starts_with("| Type | Description |")
            {
                header_seen = true;
                owner = None;
            } else if header_seen && (line.starts_with("Type:") || line.starts_with("| Type:")) {
                owner = super::explicit_type_label(line).filter(|_| line.contains('|'));
            }
            if index == line_index {
                return if self.section_at_line.get(index).is_some() {
                    owner
                } else {
                    None
                };
            }
        }
        None
    }

    fn row(&self, line: usize) -> Option<&SourceRow> {
        self.row_at_line
            .get(line)
            .and_then(|index| index.map(|i| &self.rows[i]))
    }

    fn section(&self, line: usize) -> Option<&SourceSection> {
        self.section_at_line
            .get(line)
            .and_then(|index| index.map(|i| &self.sections[i]))
    }
}

#[cfg(test)]
mod source_tree_tests {
    use super::SourceTree;

    #[test]
    fn preserves_heading_hierarchy_and_independent_table_rows() {
        let source = "# Specification\n## Commands\n| Type | Description |\n| --- | --- |\n| v | status |\n### Parameters\n| Parameter | Format |\n| speed | float |\n## Semantics\nProse";
        let tree = SourceTree::from_document(source);
        assert_eq!(tree.section(4).map(|s| s.heading_line), Some(1));
        assert_eq!(tree.section(7).and_then(|s| s.parent), Some(1));
        assert_eq!(tree.section(9).map(|s| s.heading_line), Some(8));
        assert_eq!(tree.row(4).map(|r| r.cells[0].as_str()), Some("v"));
        assert_eq!(tree.row(7).map(|r| r.cells[0].as_str()), Some("speed"));
        assert_ne!(tree.row(4).map(|r| r.table), tree.row(7).map(|r| r.table));
        assert_eq!(tree.tables.len(), 2);
    }

    #[test]
    fn flattened_table_continuation_inherits_only_explicit_row() {
        let source = "## Reserved\nType | Description | Parameters\nType: v | status | key\nContinuation one\nContinuation two\nType: V | request | key\nContinuation three\n## Semantics\nOrdinary prose";
        let tree = SourceTree::from_document(source);
        assert_eq!(tree.flattened_table_owner(source, 4), Some(b'v'));
        assert_eq!(tree.flattened_table_owner(source, 6), Some(b'V'));
        assert_eq!(tree.flattened_table_owner(source, 8), None);
    }

    #[test]
    fn flattened_table_requires_header_and_valid_declaration() {
        let source = "## Notes\nType: v | text\nProse\nType | Description | Parameters\nType: ? | malformed\nMore prose\nType: v | valid\nContinuation";
        let tree = SourceTree::from_document(source);
        assert_eq!(tree.flattened_table_owner(source, 2), None);
        assert_eq!(tree.flattened_table_owner(source, 5), None);
        assert_eq!(tree.flattened_table_owner(source, 7), Some(b'v'));
    }

    #[test]
    fn code_fences_and_prose_never_become_table_rows() {
        let source = "# Example\n```text\n| not | a table |\n```\nType: v | status\nprose\nType: V | request";
        let tree = SourceTree::from_document(source);
        assert!(tree.row(2).is_none());
        assert!(tree.row(5).is_none());
        assert_eq!(tree.row(4).map(|r| r.cells[0].as_str()), Some("Type: v"));
        assert_ne!(tree.row(4).map(|r| r.table), tree.row(6).map(|r| r.table));
    }
}

/// Source tree projected onto physical lines. Headings delimit sections;
/// explicit table rows and fenced examples establish ownership within them.
/// No ownership is inferred from candidate filenames or semantic keywords.
struct SourceStructure {
    tree: SourceTree,
    table_owners: Vec<Option<u8>>,
    example_owners: Vec<Option<u8>>,
    explicit_table_owners: Vec<Option<u8>>,
    table_row_owners: Vec<(std::ops::Range<usize>, u8)>,
    // Every observed message-table row, including rows without a declared type.
    // An untyped row is a boundary, not permission to use line-based fallback.
    table_row_ranges: Vec<std::ops::Range<usize>>,
    explicit_table_lines: Vec<bool>,
    potential_table_lines: Vec<bool>,
}

impl SourceStructure {
    fn from_document(document: &str) -> Self {
        let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(document);
        Self::from_analysis(document, &analysis)
    }

    fn from_analysis(document: &str, analysis: &simplelexer::tablelexer::DocumentAnalysis) -> Self {
        let mut owners = Vec::new();
        let mut table_owners = Vec::new();
        let mut in_message_table = false;
        let mut table_owner = None;
        let mut in_pipe_table = false;
        let mut in_examples_section = false;
        let mut in_fence = false;
        let mut example_fence = false;
        let mut previous_command: Option<u8> = None;

        let lines: Vec<&str> = document.lines().collect();
        // Prefilter explicit tables once with simplelexer. Keep pipe-bearing
        // lines as candidates because PDF conversion may have lost separators.
        let mut potential_table_lines = vec![false; lines.len()];
        let mut explicit_table_owners = vec![None; lines.len()];
        let mut table_row_owners = Vec::new();
        let mut table_row_ranges = Vec::new();
        let mut explicit_table_lines = vec![false; lines.len()];
        for block in &analysis.blocks {
            let simplelexer::tablelexer::DocumentChunk::Table(table) = block else {
                continue;
            };
            // Resolve table rows through Simplelexer's format-independent
            // logical grid. Source spans are authoritative even when multiple
            // rows occupy the same physical line.
            let grid = table.audit_grid();
            let cell_at = |row_index: usize, column: usize| {
                grid.placements.iter().find_map(|placement| {
                    (placement.column == column
                        && placement.row <= row_index
                        && row_index < placement.row + placement.rowspan)
                        .then(|| &table.rows[placement.row].cells[placement.cell])
                })
            };
            let is_message_table = (0..table.rows.len()).any(|row_index| {
                cell_at(row_index, 0).is_some_and(|cell| cell.text.trim() == "Type")
                    && grid.placements.iter().any(|placement| {
                        placement.row == row_index
                            && table.rows[placement.row].cells[placement.cell].text.trim()
                                == "Description"
                    })
            });
            if is_message_table {
                for (row_index, row) in table.rows.iter().enumerate() {
                    table_row_ranges.push(row.span.start..row.span.end);
                    let declared = cell_at(row_index, 0).and_then(|cell| {
                        let text = cell.text.trim();
                        (text.len() == 1 && text.as_bytes()[0].is_ascii_alphabetic())
                            .then(|| text.as_bytes()[0])
                    });
                    if let Some(letter) = declared {
                        table_row_owners.push((row.span.start..row.span.end, letter));
                    }
                }
            }
            let start = document[..table.span.start]
                .bytes()
                .filter(|&byte| byte == b'\n')
                .count();
            let end = document[..table.span.end]
                .bytes()
                .filter(|&byte| byte == b'\n')
                .count()
                + usize::from(
                    table.span.end > 0 && document.as_bytes()[table.span.end - 1] != b'\n',
                );
            // A lexer-recognized table has an explicit boundary. Resolve
            // declarations only within that table, never across adjacent ones.
            let mut owner = None;
            for line in start..end.min(lines.len()) {
                let raw = lines[line].trim();
                if let Some(letter) = super::explicit_type_label(raw) {
                    owner = Some(letter);
                }
                potential_table_lines[line] = true;
                explicit_table_lines[line] = true;
                explicit_table_owners[line] = owner;
            }
        }
        for (index, raw) in lines.iter().enumerate() {
            let line = raw.trim();
            if line.contains('|') {
                potential_table_lines[index] = true;
            }
            if line.starts_with('#') && !in_fence {
                in_message_table = line
                    .trim_start_matches('#')
                    .trim()
                    .to_ascii_lowercase()
                    .contains("message types");
                table_owner = None;
                in_pipe_table = false;
                in_examples_section = line
                    .trim_start_matches('#')
                    .trim()
                    .to_ascii_lowercase()
                    .contains("messages from");
            }
            if line.starts_with("```") {
                table_owner = None;
                in_pipe_table = false;
                table_owners.push(None);
                if in_fence {
                    in_fence = false;
                    example_fence = false;
                } else {
                    in_fence = true;
                    example_fence = in_examples_section
                        && matches!(line.strip_prefix("```").unwrap_or(""), "txt" | "text" | "");
                }
                previous_command = None;
                owners.push(None);
                continue;
            }
            // A row is owned only inside a declared message-type table.
            // In converted/flattened tables blank lines can occur inside cells;
            // Markdown pipe tables instead end at their first non-row.
            if !in_fence && in_message_table {
                if is_table_caption(line) || line.starts_with("| Type") || line.starts_with("| ---")
                {
                    table_owner = None;
                    in_pipe_table = line.starts_with('|');
                } else if line.starts_with("Type:") || line.starts_with("| Type:") {
                    table_owner = super::explicit_type_label(line).filter(|_| line.contains('|'));
                    in_pipe_table = line.starts_with('|');
                } else if in_pipe_table && !line.starts_with('|') && !line.is_empty() {
                    table_owner = None;
                    in_pipe_table = false;
                }
                table_owners.push(table_owner);
            } else {
                table_owners.push(None);
            }
            if !in_fence || !example_fence {
                owners.push(None);
                continue;
            }

            let bytes = line.as_bytes();
            let command = if bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1].is_ascii_whitespace()
                && (bytes[2..].contains(&b'#')
                    || lines.get(index + 1).is_some_and(|next| {
                        let next = next.trim();
                        next.contains('#')
                            && next.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                            && !next.as_bytes().get(1).is_some_and(u8::is_ascii_whitespace)
                    })) {
                Some(bytes[0])
            } else {
                None
            };

            // Only a syntactic key=value parameter can continue a command.
            // A comment or ordinary prose is not evidence of ownership.
            let first_token = line.split_whitespace().next().unwrap_or("");
            let parameter = first_token.split_once('=').is_some_and(|(key, value)| {
                !key.is_empty()
                    && !value.is_empty()
                    && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            });
            let continuation = if command.is_none()
                && previous_command.is_some()
                && parameter
                && line.contains('#')
            {
                previous_command
            } else {
                None
            };
            owners.push(command.or(continuation));
            previous_command = command;
        }
        Self {
            tree: SourceTree::from_document(document),
            table_owners,
            example_owners: owners,
            potential_table_lines,
            explicit_table_owners,
            table_row_owners,
            table_row_ranges,
            explicit_table_lines,
        }
    }

    /// Translate original table-row ownership across normalization. Ordinary prose,
    /// headings and table-free documents retain the existing resolver path.
    fn with_original_table_rows(
        mut self,
        original: &str,
        pre_normalized: &str,
        original_analysis: &simplelexer::tablelexer::DocumentAnalysis,
        row_spans: &[(std::ops::Range<usize>, std::ops::Range<usize>)],
    ) -> Self {
        let original_structure = Self::from_analysis(original, original_analysis);
        for (source_span, normalized_span) in row_spans {
            // Masking can change offsets. Never apply an unverified mapping.
            if original.get(source_span.clone()) != pre_normalized.get(source_span.clone()) {
                continue;
            }
            if original_structure
                .table_row_ranges
                .iter()
                .any(|range| range.start == source_span.start && range.end == source_span.end)
            {
                self.table_row_ranges.push(normalized_span.clone());
            }
            if let Some((_, owner)) = original_structure
                .table_row_owners
                .iter()
                .find(|(range, _)| range.start == source_span.start && range.end == source_span.end)
            {
                self.table_row_owners
                    .push((normalized_span.clone(), *owner));
            }
        }
        self
    }

    fn is_message_table_row_at(&self, byte_offset: usize) -> bool {
        self.table_row_ranges
            .iter()
            .any(|range| range.contains(&byte_offset))
    }

    fn table_row_type_at(&self, byte_offset: usize) -> Option<u8> {
        self.table_row_owners
            .iter()
            .find(|(range, _)| range.contains(&byte_offset))
            .map(|(_, letter)| *letter)
    }

    fn is_explicit_table_line(&self, line_index: usize) -> bool {
        self.explicit_table_lines
            .get(line_index)
            .copied()
            .unwrap_or(false)
    }

    fn explicit_table_type(&self, line_index: usize) -> Option<u8> {
        self.explicit_table_owners
            .get(line_index)
            .copied()
            .flatten()
    }

    fn is_potential_table_line(&self, line_index: usize) -> bool {
        self.potential_table_lines
            .get(line_index)
            .copied()
            .unwrap_or(false)
    }

    fn table_type(&self, line_index: usize) -> Option<u8> {
        // The tree records row/section boundaries independently of ownership.
        let _structural_row = self.tree.row(line_index);
        let _structural_section = self.tree.section(line_index);
        self.table_owners.get(line_index).copied().flatten()
    }

    fn example_type(&self, line_index: usize) -> Option<u8> {
        self.example_owners.get(line_index).copied().flatten()
    }
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
#[cfg(test)]
fn resolve_fact_type_in_document(
    window: &str,
    window_start: usize,
    document: &str,
    fact: &CandidateFact,
) -> Result<u8, ResolutionFailure> {
    let structure = SourceStructure::from_document(document);
    resolve_fact_type_with_flattened_tables(window, window_start, document, fact, true, &structure)
}

fn resolve_fact_type_with_flattened_tables(
    window: &str,
    window_start: usize,
    document: &str,
    fact: &CandidateFact,
    include_flattened_tables: bool,
    structure: &SourceStructure,
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
    // The HTML row is a stronger source locator than its physical line:
    // PDF conversions frequently put an entire multi-message table on one line.
    if let Some(letter) = structure.table_row_type_at(absolute) {
        if super::explicit_type_label(&fact.statement).is_some_and(|stated| stated != letter) {
            return Err(ResolutionFailure::ConflictingType);
        }
        if let Ok(local_letter) = local
            && local_letter != letter
        {
            return Err(ResolutionFailure::ConflictingType);
        }
        return Ok(letter);
    }
    // A recognized HTML message row without a type must not inherit an
    // unrelated declaration from the surrounding physical source line.
    if structure.is_message_table_row_at(absolute) {
        return Err(ResolutionFailure::NoSourceType);
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
        structure.explicit_table_type(line).or_else(|| {
            (structure.is_potential_table_line(line) && !structure.is_explicit_table_line(line))
                .then(|| table_block_type_for_line(document, line))
                .flatten()
        }),
        structure.table_type(line),
        include_flattened_tables
            .then(|| structure.tree.flattened_table_owner(document, line))
            .flatten(),
        message_example_type_for_line(document, line),
        structure.example_type(line),
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
mod html_row_identity_tests {
    use super::*;

    #[test]
    fn separate_messages_on_one_html_line_keep_case() {
        let document = concat!(
            "## Message Types\n",
            "<table><tr><td>Type</td><td>Description</td></tr>",
            "<tr><td>B</td><td>upper</td></tr>",
            "<tr><td>b</td><td>lower</td></tr></table>"
        );
        let structure = SourceStructure::from_document(document);
        for (letter, description) in [(b'B', "upper"), (b'b', "lower")] {
            let offset = document.find(description).unwrap();
            let fact = CandidateFact {
                statement: description.to_owned(),
                segment_id: String::new(),
                evidence_quote: Some(description.to_owned()),
                source_byte_offset: Some(offset),
                source_locator_ambiguous: false,
            };
            assert_eq!(
                resolve_fact_type_with_flattened_tables(
                    document, 0, document, &fact, true, &structure
                ),
                Ok(letter)
            );
        }
    }

    #[test]
    fn normalized_rows_keep_original_case_and_table_free_fallback() {
        let original = concat!(
            "## Message Types\n",
            "<table><tr><td>Type</td><td>Description</td></tr>",
            "<tr><td>B</td><td>upper evidence</td></tr>",
            "<tr><td>b</td><td>lower evidence</td></tr></table>\n",
            "## Other\nplain paragraph"
        );
        let (normalized, spans) =
            crate::utils::stringutils::normalize_html_tables_with_spans(original);
        let original_analysis = simplelexer::tablelexer::DocumentAnalysis::parse(original);
        let normalized_analysis = simplelexer::tablelexer::DocumentAnalysis::parse(&normalized);
        let structure = SourceStructure::from_analysis(&normalized, &normalized_analysis)
            .with_original_table_rows(original, original, &original_analysis, &spans);
        for (letter, evidence) in [(b'B', "upper evidence"), (b'b', "lower evidence")] {
            let offset = normalized.find(evidence).unwrap();
            assert_eq!(structure.table_row_type_at(offset), Some(letter));
        }
        assert_eq!(
            structure.table_row_type_at(normalized.find("plain paragraph").unwrap()),
            None
        );
        let table_free = "## Message Types\nplain paragraph";
        let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(table_free);
        let fallback = SourceStructure::from_analysis(table_free, &analysis)
            .with_original_table_rows(table_free, table_free, &analysis, &[]);
        assert_eq!(
            fallback.table_row_type_at(table_free.find("paragraph").unwrap()),
            None
        );
    }

    #[test]
    fn empty_html_type_cell_does_not_panic_or_claim_identity() {
        let document = concat!(
            "## Message Types\n",
            "<table><tr><td>Type</td><td>Description</td></tr>",
            "<tr><td></td><td>untyped evidence</td></tr>",
            "<tr><td>B</td><td>typed evidence</td></tr></table>"
        );
        // Parsing the original HTML exercises the empty-cell path that
        // previously panicked; positive row ownership is covered separately.
        let structure = SourceStructure::from_document(document);
        assert_eq!(
            structure.table_row_type_at(document.find("untyped evidence").unwrap()),
            None
        );
    }

    #[test]
    fn untyped_html_row_blocks_line_fallback_after_typed_row() {
        let document = concat!(
            "## Message Types\n",
            "<table><tr><td>Type</td><td>Description</td></tr>",
            "<tr><td>B</td><td>typed evidence</td></tr>",
            "<tr><td></td><td>untyped evidence</td></tr></table>"
        );
        let structure = SourceStructure::from_document(document);
        let offset = document.find("untyped evidence").unwrap();
        let fact = CandidateFact {
            statement: "untyped evidence".to_owned(),
            segment_id: String::new(),
            evidence_quote: Some("untyped evidence".to_owned()),
            source_byte_offset: Some(offset),
            source_locator_ambiguous: false,
        };
        assert!(structure.is_message_table_row_at(offset));
        assert_eq!(
            resolve_fact_type_with_flattened_tables(document, 0, document, &fact, true, &structure),
            Err(ResolutionFailure::NoSourceType)
        );
    }

    #[test]
    fn normalized_untyped_html_row_remains_a_boundary() {
        let original = concat!(
            "## Message Types\n",
            "<table><tr><td>Type</td><td>Description</td></tr>",
            "<tr><td>B</td><td>typed evidence</td></tr>",
            "<tr><td></td><td>untyped evidence</td></tr></table>"
        );
        let (normalized, spans) =
            crate::utils::stringutils::normalize_html_tables_with_spans(original);
        let original_analysis = simplelexer::tablelexer::DocumentAnalysis::parse(original);
        let normalized_analysis = simplelexer::tablelexer::DocumentAnalysis::parse(&normalized);
        let structure = SourceStructure::from_analysis(&normalized, &normalized_analysis)
            .with_original_table_rows(original, original, &original_analysis, &spans);
        let offset = normalized.find("untyped evidence").unwrap();
        assert!(structure.is_message_table_row_at(offset));
        assert_eq!(structure.table_row_type_at(offset), None);
        assert!(!structure.is_message_table_row_at(normalized.len()));
    }

    #[test]
    fn adjacent_html_tables_do_not_share_row_ownership() {
        let document = concat!(
            "## Message Types\n",
            "<table><tr><td>Type</td><td>Description</td></tr>",
            "<tr><td>B</td><td>first message</td></tr></table>\n",
            "<table><tr><td>Type</td><td>Description</td></tr>",
            "<tr><td></td><td>second untyped</td></tr></table>\n",
            "## Other\nordinary prose"
        );
        let structure = SourceStructure::from_document(document);
        let typed = document.find("first message").unwrap();
        let untyped = document.find("second untyped").unwrap();
        let prose = document.find("ordinary prose").unwrap();
        assert_eq!(structure.table_row_type_at(typed), Some(b'B'));
        assert!(structure.is_message_table_row_at(untyped));
        assert_eq!(structure.table_row_type_at(untyped), None);
        assert!(!structure.is_message_table_row_at(prose));
        assert_eq!(structure.table_row_type_at(prose), None);
    }

    #[test]
    fn html_row_with_conflicting_fact_type_is_rejected() {
        let document = concat!(
            "## Message Types\n",
            "<table><tr><td>Type</td><td>Description</td></tr>",
            "<tr><td>B</td><td>typed evidence</td></tr></table>"
        );
        let structure = SourceStructure::from_document(document);
        let offset = document.find("typed evidence").unwrap();
        let fact = CandidateFact {
            statement: "Type: b | typed evidence".to_owned(),
            segment_id: String::new(),
            evidence_quote: Some("typed evidence".to_owned()),
            source_byte_offset: Some(offset),
            source_locator_ambiguous: false,
        };
        assert_eq!(
            resolve_fact_type_with_flattened_tables(document, 0, document, &fact, true, &structure),
            Err(ResolutionFailure::ConflictingType)
        );
    }

    #[test]
    fn description_rowspan_never_inherits_message_type() {
        // A rowspan in the Description column is structural grouping, not
        // evidence that an empty Type cell belongs to the previous message.
        let document = concat!(
            "## Message Types\n",
            "<table><tr><td>Type</td><td>Description</td><td>Parameters</td></tr>",
            "<tr><td>y</td><td>typed response</td><td>replyTo=getConfigB</td></tr>",
            "<tr><td></td><td rowspan=\"3\">getConfigB response</td><td>first detail</td></tr>",
            "<tr><td></td><td>second detail</td></tr>",
            "<tr><td></td><td>third detail</td></tr></table>"
        );
        let structure = SourceStructure::from_document(document);
        assert_eq!(
            structure.table_row_type_at(document.find("typed response").unwrap()),
            Some(b'y')
        );
        for evidence in ["first detail", "second detail", "third detail"] {
            let offset = document.find(evidence).unwrap();
            assert!(structure.is_message_table_row_at(offset));
            assert_eq!(structure.table_row_type_at(offset), None);
            let fact = CandidateFact {
                statement: evidence.to_owned(),
                segment_id: String::new(),
                evidence_quote: Some(evidence.to_owned()),
                source_byte_offset: Some(offset),
                source_locator_ambiguous: false,
            };
            assert_eq!(
                resolve_fact_type_with_flattened_tables(
                    document, 0, document, &fact, true, &structure
                ),
                Err(ResolutionFailure::NoSourceType)
            );
        }
    }

    #[test]
    fn unrelated_html_tables_do_not_declare_message_types() {
        let document = "<table><tr><td>Name</td><td>Description</td></tr><tr><td>B</td><td>ordinary</td></tr></table>";
        let structure = SourceStructure::from_document(document);
        assert_eq!(
            structure.table_row_type_at(document.find("ordinary").unwrap()),
            None
        );
    }
}

#[cfg(test)]
mod source_structure_tests {
    use super::SourceStructure;

    #[test]
    fn table_tree_owns_flattened_rows_and_respects_next_declaration() {
        let source = "## Message Types\nType: v | status | value\ncontinuation\nmore details\nType: V | request | key\nmore details\n## Semantics\nnot a row";
        let tree = SourceStructure::from_document(source);
        assert_eq!(tree.table_type(3), Some(b'v'));
        assert_eq!(tree.table_type(5), Some(b'V'));
        assert_eq!(tree.table_type(7), None);
    }

    #[test]
    fn table_tree_does_not_inherit_malformed_declarations_or_pipe_prose() {
        let source = "## Message Types\nType: v | status\nType: ?? | unknown\ncontinuation\n## Other\n| ordinary | prose |";
        let tree = SourceStructure::from_document(source);
        assert_eq!(tree.table_type(1), Some(b'v'));
        assert_eq!(tree.table_type(3), None);
        assert_eq!(tree.table_type(5), None);
    }

    #[test]
    fn owns_immediate_continuation_of_fenced_command() {
        let source = "## Messages from Modem to Antenna\n```txt\nB 100 200 rx2=300\ntx3=400 # additional frequency\n```";
        let structure = SourceStructure::from_document(source);
        assert_eq!(structure.example_type(2), Some(b'B'));
        assert_eq!(structure.example_type(3), Some(b'B'));
    }

    #[test]
    fn never_inherits_across_blank_lines_fences_or_sections() {
        let source = "## Messages from Modem to Antenna\n```txt\nB 100 # example\n\ntx3=400 # detached\n```\n## Appendix\n```txt\nB 100 # unrelated\n```";
        let structure = SourceStructure::from_document(source);
        assert_eq!(structure.example_type(4), None);
        assert_eq!(structure.example_type(8), None);
    }

    #[test]
    fn preserves_case_and_does_not_claim_arbitrary_prose() {
        let source = "## Messages from Modem to Antenna\n```txt\nb 100 # command\nplain prose # comment\nQ 20 # different command\n```";
        let structure = SourceStructure::from_document(source);
        assert_eq!(structure.example_type(2), Some(b'b'));
        assert_eq!(structure.example_type(3), None);
        assert_eq!(structure.example_type(4), Some(b'Q'));
    }
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
    let structure = SourceStructure::from_document(document);
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
                resolve_fact_type_with_flattened_tables(
                    window, *start, document, fact, true, &structure
                ),
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
    let structure = SourceStructure::from_document(document);
    let mut result = UnresolvedTriage::default();
    for candidate in candidates {
        let Some((window, start)) = source_window_index(&candidate.id)
            .and_then(|index| sources.get(index).zip(starts.get(index)))
        else {
            continue;
        };
        for fact in &candidate.facts {
            if !matches!(
                resolve_fact_type_with_flattened_tables(
                    window, *start, document, fact, true, &structure
                ),
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
    let structure = SourceStructure::from_document(document);
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
                resolve_fact_type_with_flattened_tables(
                    window, *start, document, fact, true, &structure
                ),
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

pub(super) struct DocumentIdentityContext<'a> {
    pub document: &'a str,
    pub analysis: &'a simplelexer::tablelexer::DocumentAnalysis<'a>,
    pub original: &'a str,
    pub pre_normalized: &'a str,
    pub original_analysis: &'a simplelexer::tablelexer::DocumentAnalysis<'a>,
    pub row_spans: &'a crate::utils::stringutils::HtmlRowSpans,
}

pub(super) fn resolution_diagnostics_in_document(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    context: &DocumentIdentityContext<'_>,
) -> ResolutionDiagnostics {
    let document = context.document;
    let structure = SourceStructure::from_analysis(document, context.analysis)
        .with_original_table_rows(
            context.original,
            context.pre_normalized,
            context.original_analysis,
            context.row_spans,
        );
    let mut result = ResolutionDiagnostics::default();
    for candidate in candidates {
        let window = source_window_index(&candidate.id);
        for fact in &candidate.facts {
            let Some((source, start)) = window.and_then(|i| sources.get(i).zip(starts.get(i)))
            else {
                result.invalid_window += 1;
                continue;
            };
            match resolve_fact_type_with_flattened_tables(
                source, *start, document, fact, true, &structure,
            ) {
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

/// Replay-only audit of assignments removed by the untyped HTML-row boundary.
/// Compares the same resolver with and without that boundary; no projection changes.
pub(super) fn html_boundary_blocked_assignments(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    context: &DocumentIdentityContext<'_>,
) -> Vec<String> {
    let document = context.document;
    let structure = SourceStructure::from_analysis(document, context.analysis)
        .with_original_table_rows(
            context.original,
            context.pre_normalized,
            context.original_analysis,
            context.row_spans,
        );
    let mut legacy = SourceStructure::from_analysis(document, context.analysis)
        .with_original_table_rows(
            context.original,
            context.pre_normalized,
            context.original_analysis,
            context.row_spans,
        );
    legacy.table_row_ranges.clear();
    let mut unique = std::collections::BTreeMap::new();
    for candidate in candidates {
        let Some(window) = source_window_index(&candidate.id) else {
            continue;
        };
        let Some((source, start)) = sources.get(window).zip(starts.get(window)) else {
            continue;
        };
        for fact in &candidate.facts {
            let Some(offset) = fact.source_byte_offset else {
                continue;
            };
            let Some(absolute) = start.checked_add(offset) else {
                continue;
            };
            if !structure.is_message_table_row_at(absolute)
                || structure.table_row_type_at(absolute).is_some()
            {
                continue;
            }
            if resolve_fact_type_with_flattened_tables(
                source, *start, document, fact, true, &structure,
            ) != Err(ResolutionFailure::NoSourceType)
            {
                continue;
            }
            let Ok(old_type) = resolve_fact_type_with_flattened_tables(
                source, *start, document, fact, true, &legacy,
            ) else {
                continue;
            };
            let line = document.as_bytes()[..absolute]
                .iter()
                .filter(|&&byte| byte == b'\n')
                .count()
                + 1;
            let key = (
                absolute,
                fact.statement.clone(),
                fact.evidence_quote.clone(),
            );
            unique.entry(key).or_insert_with(|| {
                format!(
                    "line={line} byte={absolute} previous_type={} statement={:?} evidence={:?}",
                    old_type as char,
                    fact.statement.chars().take(160).collect::<String>(),
                    fact.evidence_quote
                        .as_deref()
                        .unwrap_or("")
                        .chars()
                        .take(160)
                        .collect::<String>()
                )
            });
        }
    }
    unique.into_values().collect()
}

/// Replay-only comparison against the previous HTML-only table-row resolver.
/// Reports source-backed assignments, not candidate occurrence counts.
pub(super) fn generic_table_identity_gains(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    context: &DocumentIdentityContext<'_>,
) -> Vec<String> {
    let current = SourceStructure::from_analysis(context.document, context.analysis)
        .with_original_table_rows(
            context.original,
            context.pre_normalized,
            context.original_analysis,
            context.row_spans,
        );
    let mut previous = SourceStructure::from_analysis(context.document, context.analysis)
        .with_original_table_rows(
            context.original,
            context.pre_normalized,
            context.original_analysis,
            context.row_spans,
        );
    // The former resolver used row ownership only for HTML tables.
    // Preserve normalized rows mapped from original HTML, but remove rows
    // discovered solely through Markdown table parsing.
    let translated: std::collections::HashSet<(usize, usize)> = context
        .row_spans
        .iter()
        .map(|(_, normalized)| (normalized.start, normalized.end))
        .collect();
    let markdown_rows: std::collections::HashSet<(usize, usize)> = context
        .analysis
        .blocks
        .iter()
        .filter_map(|block| match block {
            simplelexer::tablelexer::DocumentChunk::Table(table)
                if table.table_type == simplelexer::tablelexer::TableType::Markdown =>
            {
                Some(table.rows.iter().map(|row| (row.span.start, row.span.end)))
            }
            _ => None,
        })
        .flatten()
        .filter(|span| !translated.contains(span))
        .collect();
    previous
        .table_row_ranges
        .retain(|range| !markdown_rows.contains(&(range.start, range.end)));
    previous
        .table_row_owners
        .retain(|(range, _)| !markdown_rows.contains(&(range.start, range.end)));

    let mut gains = std::collections::BTreeMap::new();
    for candidate in candidates {
        let Some(window) = source_window_index(&candidate.id) else {
            continue;
        };
        let Some((source, start)) = sources.get(window).zip(starts.get(window)) else {
            continue;
        };
        for fact in &candidate.facts {
            let Ok(new_type) = resolve_fact_type_with_flattened_tables(
                source,
                *start,
                context.document,
                fact,
                true,
                &current,
            ) else {
                continue;
            };
            let old = resolve_fact_type_with_flattened_tables(
                source,
                *start,
                context.document,
                fact,
                true,
                &previous,
            );
            if old == Ok(new_type) {
                continue;
            }
            let Some(absolute) = fact
                .source_byte_offset
                .and_then(|offset| start.checked_add(offset))
            else {
                continue;
            };
            let Some((range, row_type)) = current
                .table_row_ranges
                .iter()
                .find(|range| range.contains(&absolute))
                .map(|range| (range, current.table_row_type_at(absolute)))
            else {
                continue;
            };
            let line = context.document.as_bytes()[..absolute]
                .iter()
                .filter(|&&byte| byte == b'\n')
                .count()
                + 1;
            let key = (
                absolute,
                fact.statement.clone(),
                fact.evidence_quote.clone(),
            );
            gains.entry(key).or_insert_with(|| format!(
                "line={line} byte={absolute} row={}..{} row_type={:?} previous={old:?} new_type={} statement={:?} evidence={:?}",
                range.start, range.end, row_type.map(char::from),
                char::from(new_type),
                fact.statement.chars().take(140).collect::<String>(),
                fact.evidence_quote.as_deref().unwrap_or("")
                    .chars().take(140).collect::<String>(),
            ));
        }
    }
    gains.into_values().collect()
}

/// Read-only diff of facts newly resolved by flattened table ownership.
/// One line per unique source-backed fact; duplicate candidate occurrences
/// do not inflate the reported gain.
pub(super) fn flattened_table_identity_gains(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    document: &str,
) -> Vec<String> {
    let structure = SourceStructure::from_document(document);
    let mut gains = std::collections::BTreeMap::new();
    let lines: Vec<&str> = document.lines().collect();
    let tree = SourceTree::from_document(document);
    for candidate in candidates {
        let Some(window_index) = source_window_index(&candidate.id) else {
            continue;
        };
        let Some((window, start)) = sources.get(window_index).zip(starts.get(window_index)) else {
            continue;
        };
        for fact in &candidate.facts {
            let Ok(letter) = resolve_fact_type_with_flattened_tables(
                window, *start, document, fact, true, &structure,
            ) else {
                continue;
            };
            if !matches!(
                resolve_fact_type_with_flattened_tables(
                    window, *start, document, fact, false, &structure
                ),
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
            let Some(line_index) =
                crate::utils::stringutils::line_index_at_byte_offset(document, absolute)
            else {
                continue;
            };
            let Some(source_line) = lines.get(line_index) else {
                continue;
            };
            let Some(owner) = tree.flattened_table_owner(document, line_index) else {
                continue;
            };
            if owner != letter {
                continue;
            }
            let mut declaration = None;
            for (i, raw) in lines.iter().enumerate().take(line_index + 1).rev() {
                let line = raw.trim();
                if line.starts_with('#') || is_table_caption(line) {
                    break;
                }
                if super::explicit_type_label(line) == Some(letter) && line.contains('|') {
                    declaration = Some(i + 1);
                    break;
                }
            }
            let evidence = fact.evidence_quote.as_deref().unwrap_or("");
            let snippet =
                |s: &str| -> String { s.chars().take(110).collect::<String>().replace('\n', " ") };
            gains.entry((absolute, fact.statement.clone(), evidence.to_owned()))
                .or_insert_with(|| format!(
                    "line={} type={} declaration_line={} reason=flattened_table_continuation statement={:?} source={:?}",
                    line_index + 1,
                    letter as char,
                    declaration.map_or("unknown".into(), |n| n.to_string()),
                    snippet(&fact.statement),
                    snippet(source_line)
                ));
        }
    }
    gains.into_values().collect()
}

pub(super) fn resolved_source_facts_in_document(
    candidates: &[MessageCandidate],
    sources: &[&str],
    starts: &[usize],
    context: &DocumentIdentityContext<'_>,
) -> (
    std::collections::BTreeMap<String, Vec<(usize, CandidateFact)>>,
    usize,
) {
    let document = context.document;
    let structure = SourceStructure::from_analysis(document, context.analysis)
        .with_original_table_rows(
            context.original,
            context.pre_normalized,
            context.original_analysis,
            context.row_spans,
        );
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
            match resolve_fact_type_with_flattened_tables(
                source, *start, document, fact, true, &structure,
            ) {
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
