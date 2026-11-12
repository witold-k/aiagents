// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Diagnostic, language-neutral protocol model staging.
//! This is an inventory of hypotheses and source anchors, not a wire schema.

use super::llm_structure::{SemanticUnit, SourceGraph};
use super::message_identity::{MessageCandidate, source_window_index};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct ProtocolModel {
    schema_version: u32,
    status: &'static str,
    messages: Vec<MessageHypothesis>,
    table_evidence: Vec<TableEvidence>,
    semantic_units: Vec<SemanticUnitEvidence>,
    hypothetical_semantic_units: Vec<SemanticUnitEvidence>,
    reconstructed_table_entries: Vec<ReconstructedTableEntry>,
    table_repair_hypotheses: Vec<TableRepairHypothesis>,
    assembled_parameters: Vec<AssembledParameter>,
    unassembled_semantic_rows: Vec<UnassembledSemanticRow>,
    caveats: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct TableEvidence {
    normalized_byte_start: usize,
    normalized_byte_end: usize,
    rows: Vec<TableRowEvidence>,
}

#[derive(Debug, Serialize)]
struct TableRowEvidence {
    normalized_byte_start: usize,
    normalized_byte_end: Option<usize>,
    cells: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SemanticUnitEvidence {
    entry_normalized_byte: usize,
    row_normalized_bytes: Vec<usize>,
    rows: Vec<TableRowEvidence>,
    supporting_relations: Vec<RelationEvidence>,
    status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct RelationEvidence {
    earlier_normalized_byte: usize,
    later_normalized_byte: usize,
    relation: String,
    votes: usize,
    valid_attempts: usize,
}

impl ProtocolModel {
    /// Attach only existing, voted row relations; never infer field ownership.
    pub(super) fn with_semantic_units(
        mut self,
        graph: &SourceGraph,
        units: &[SemanticUnit],
        hypothetical: &[SemanticUnit],
    ) -> Self {
        self.semantic_units = project_semantic_units(graph, units, "proposed_voted_continuation");
        self.hypothetical_semantic_units =
            project_semantic_units(graph, hypothetical, "hypothetical_voted_boundary_without_lexical_anchor");

        self.reconstructed_table_entries = reconstruct_table_entries(&self.semantic_units);
        self.table_repair_hypotheses = table_repair_hypotheses(&self.semantic_units);
        self.unassembled_semantic_rows = self.semantic_units.iter()
            .flat_map(|unit| &unit.rows)
            .filter(|row| !row.cells.iter().any(|cell| declared_parameter(cell).is_some()))
            .map(|row| UnassembledSemanticRow {
                normalized_byte_start: row.normalized_byte_start,
                cells: row.cells.clone(),
                reason: "no_explicit_angle_bracket_declaration",
                message_owner: None,
            })
            .collect();
        self.assembled_parameters = self
            .semantic_units
            .iter()
            .flat_map(assemble_unit)
            .collect();
        self
    }
}

/// Project source rows and voted relations without promoting lexical or hypothetical
/// units to verified parameters or message ownership.
fn project_semantic_units(
    graph: &SourceGraph,
    units: &[SemanticUnit],
    status: &'static str,
) -> Vec<SemanticUnitEvidence> {
    units.iter().filter_map(|unit| {
        let rows: Option<Vec<_>> = unit.row_starts.iter().map(|start| {
            graph.rows.iter().find(|row| row.start == *start).map(|row| TableRowEvidence {
                normalized_byte_start: row.start,
                normalized_byte_end: None,
                cells: row.cells.clone(),
            })
        }).collect();
        let relations: Option<Vec<_>> = unit.supporting_relations.iter().map(|index| {
            graph.relations.get(*index).map(|r| RelationEvidence {
                earlier_normalized_byte: r.earlier_row,
                later_normalized_byte: r.later_row,
                relation: r.relation.clone(),
                votes: r.votes,
                valid_attempts: r.valid_attempts,
            })
        }).collect();
        Some(SemanticUnitEvidence {
            entry_normalized_byte: unit.entry_row,
            row_normalized_bytes: unit.row_starts.clone(),
            rows: rows?,
            supporting_relations: relations?,
            status,
        })
    }).collect()
}

/// Read-only reconstruction of the observed cell sequence in a voted unit.
/// The content is intentionally not parsed into parameter names, types or owners.
#[derive(Debug, Serialize)]
struct ReconstructedTableEntry {
    entry_normalized_byte: usize,
    row_normalized_bytes: Vec<usize>,
    first_column_fragments: Vec<String>,
    second_column_fragments: Vec<String>,
    supporting_relations: Vec<RelationEvidence>,
    status: &'static str,
    message_owner: Option<String>,
}

fn reconstruct_table_entries(units: &[SemanticUnitEvidence]) -> Vec<ReconstructedTableEntry> {
    units.iter().filter(|unit| unit.rows.len() > 1)
        .map(|unit| ReconstructedTableEntry {
            entry_normalized_byte: unit.entry_normalized_byte,
            row_normalized_bytes: unit.row_normalized_bytes.clone(),
            first_column_fragments: unit.rows.iter()
                .map(|row| row.cells.first().cloned().unwrap_or_default()).collect(),
            second_column_fragments: unit.rows.iter()
                .map(|row| row.cells.get(1).cloned().unwrap_or_default()).collect(),
            supporting_relations: unit.supporting_relations.clone(),
            status: "voted_row_sequence_uninterpreted",
            message_owner: None,
        }).collect()
}

/// Alternative interpretations of damaged cell layout, not repaired source text.
#[derive(Debug, Serialize)]
struct TableRepairHypothesis {
    entry_normalized_byte: usize,
    row_normalized_bytes: Vec<usize>,
    candidate_name: Option<String>,
    candidate_type: Option<String>,
    candidate_unit: Option<String>,
    description_fragments: Vec<String>,
    first_column_prefix: Option<String>,
    declaration_source_row: usize,
    declaration_source_cell: String,
    evidence_status: &'static str,
    status: &'static str,
    message_owner: Option<String>,
}

/// Require an exact bare declaration in the second cell: a leading count,
/// type, identifier, unit. No token may be silently discarded.
fn bare_column_declaration(cell: &str) -> Option<(String, String, String)> {
    let tokens: Vec<_> = cell.split_whitespace().collect();
    let [count, ty, name, unit] = tokens.as_slice() else { return None };
    if count.parse::<usize>().ok()? == 0
        || !matches!(*ty, "int" | "string" | "float" | "double" | "bool")
        || !name.starts_with(|c: char| c.is_ascii_alphabetic())
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        || !unit.chars().all(|c| c.is_ascii_alphabetic() || c == '_')
    {
        return None;
    }
    Some(((*name).to_owned(), (*ty).to_owned(), (*unit).to_owned()))
}

fn table_repair_hypotheses(units: &[SemanticUnitEvidence]) -> Vec<TableRepairHypothesis> {
    units.iter().filter_map(|unit| {
        let first = unit.rows.first()?;
        if unit.rows.len() < 2 || unit.supporting_relations.is_empty() {
            return None;
        }
        let (name, ty, unit_hint) = bare_column_declaration(first.cells.get(1)?)?;
        // Keep the first-column prefix as raw evidence, without interpreting
        // it as a message code, parameter name, or ownership signal.
        let prefix = first.cells.first()?.split_whitespace().next().map(str::to_owned);
        Some(TableRepairHypothesis {
            entry_normalized_byte: unit.entry_normalized_byte,
            row_normalized_bytes: unit.row_normalized_bytes.clone(),
            candidate_name: Some(name),
            candidate_type: Some(ty),
            candidate_unit: Some(unit_hint),
            description_fragments: unit.rows.iter().filter_map(|row| row.cells.first().cloned()).collect(),
            first_column_prefix: prefix,
            declaration_source_row: first.normalized_byte_start,
            declaration_source_cell: first.cells[1].clone(),
            evidence_status: "exact_observed_cell_syntax",
            status: "ambiguous_column_layout_unverified",
            message_owner: None,
        })
    }).collect()
}

#[derive(Debug, Serialize)]
struct UnassembledSemanticRow {
    normalized_byte_start: usize,
    cells: Vec<String>,
    reason: &'static str,
    message_owner: Option<String>,
}

#[derive(Debug, Serialize)]
struct AssembledParameter {
    name: String,
    type_hint: Option<String>,
    range_hint: Option<String>,
    unit_hint: Option<String>,
    evidence_row_starts: Vec<usize>,
    status: &'static str,
    message_owner: Option<String>,
}

/// Extract only explicit angle-bracket declarations, never examples or prose.
/// The raw payload is retained because units and bounds are not yet validated.
fn declared_parameter(cell: &str) -> Option<(String, String, Option<String>, Option<String>)> {
    for token in cell.split_whitespace() {
        let token = token.trim_matches(|c: char| matches!(c, '|' | '`' | ',' | ';' | '*' | '(' | ')'));
        let Some((name, value)) = token.split_once('=') else { continue };
        if name.is_empty()
            || !name.starts_with(|c: char| c.is_ascii_alphabetic())
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }
        // Read the complete declaration from the original cell: its payload may
        // contain spaces (e.g. "int 0 to 65535").
        let Some(token_start) = cell.find(token) else { continue };
        let full_value = &cell[token_start + token.len() - value.len()..];
        let Some(rest) = full_value.strip_prefix('<').or_else(|| full_value.strip_prefix("&lt;")) else {
            continue;
        };
        let payload = if let Some(end) = rest.find('>') { &rest[..end] } else { rest };
        let ty = payload.split_whitespace().next().unwrap_or("");
        if ty.is_empty() || !ty.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let tail = payload[ty.len()..].trim();
        let (range, unit) = if let Some((lo, hi)) = tail.split_once(" to ") {
            if !lo.is_empty() && !hi.is_empty()
                && lo.chars().all(|c| c.is_ascii_digit())
                && hi.chars().all(|c| c.is_ascii_digit())
            {
                (Some(tail.to_owned()), None)
            } else {
                (None, None)
            }
        } else if !tail.is_empty()
            && tail.chars().all(|c| c.is_ascii_alphabetic() || c == '_')
        {
            (None, Some(tail.to_owned()))
        } else {
            (None, None)
        };
        return Some((name.to_owned(), ty.to_owned(), range, unit));
    }
    None
}

/// A unit can contain several declarations. Do not collapse conflicting ones.
fn assemble_unit(unit: &SemanticUnitEvidence) -> Vec<AssembledParameter> {
    let mut result = Vec::new();
    for row in &unit.rows {
        for cell in &row.cells {
            let Some((name, ty, range, unit_hint)) = declared_parameter(cell) else {
                continue;
            };
            result.push(AssembledParameter {
                name,
                type_hint: Some(ty),
                range_hint: range,
                unit_hint,
                evidence_row_starts: vec![row.normalized_byte_start],
                status: "unverified_lexical_assembly",
                message_owner: None,
            });
        }
    }
    result
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
    syntax_kind: &'static str,
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

/// Classify the value syntax, not the semantic meaning of a parameter.
/// Only an explicit angle-bracket type declaration is recognized.
fn assignment_syntax(quote: &str, key: &str) -> (&'static str, Option<String>, Vec<String>) {
    for token in quote.split_whitespace() {
        let clean =
            token.trim_matches(|c: char| matches!(c, '|' | '`' | ',' | ';' | '*' | '(' | ')'));
        let Some(value) = clean.strip_prefix(key).and_then(|s| s.strip_prefix('=')) else {
            continue;
        };
        let rest = if let Some(rest) = value.strip_prefix('<') {
            rest
        } else if let Some(rest) = value.strip_prefix("&lt;") {
            rest
        } else {
            return ("assignment_example_or_value", None, Vec::new());
        };
        // Type is lexical only. A range must not be guessed from free text.
        let ty = rest
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .next()
            .filter(|s| !s.is_empty());
        return match ty {
            Some(ty) => ("definition_syntax", Some(ty.to_owned()), Vec::new()),
            None => ("unclassified", None, Vec::new()),
        };
    }
    ("unclassified", None, Vec::new())
}

pub(super) fn stage(
    candidates: &[MessageCandidate],
    windows: &[&str],
    normalized_analysis: &simplelexer::tablelexer::DocumentAnalysis,
) -> ProtocolModel {
    // Copy existing lexer observations. Normalized offsets are not original
    // source offsets and must never be used to claim fact ownership.
    let table_evidence = normalized_analysis
        .blocks
        .iter()
        .filter_map(|block| {
            let simplelexer::tablelexer::DocumentChunk::Table(table) = block else {
                return None;
            };
            Some(TableEvidence {
                normalized_byte_start: table.span.start,
                normalized_byte_end: table.span.end,
                rows: table
                    .rows
                    .iter()
                    .map(|row| TableRowEvidence {
                        normalized_byte_start: row.span.start,
                        normalized_byte_end: Some(row.span.end),
                        cells: row.cells.iter().map(|cell| cell.text.to_string()).collect(),
                    })
                    .collect(),
            })
        })
        .collect();
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
                    let (syntax_kind, value_type, constraints) =
                        assignment_syntax(fact.evidence_quote.as_deref().unwrap_or_default(), key);
                    parameter_hints.push(ParameterHint {
                        name: key.to_owned(),
                        origin_assertion: index,
                        status: "lexical_hypothesis",
                        syntax_kind,
                        value_type,
                        required: None,
                        constraints,
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
        table_evidence,
        semantic_units: Vec::new(),
        hypothetical_semantic_units: Vec::new(),
        reconstructed_table_entries: Vec::new(),
        table_repair_hypotheses: Vec::new(),
        assembled_parameters: Vec::new(),
        unassembled_semantic_rows: Vec::new(),
        caveats: vec![
            "Candidates are not canonical messages; repeated names do not imply identity.",
            "Table evidence uses normalized-document byte offsets; original source mapping and message ownership are not asserted.",
            "Parameter hints distinguish lexical definition syntax from assignments; neither establishes verified fields or message ownership.",
            "Lexical type labels are not validated types; ranges, cardinality, constraints, defaults and protocol behavior are not inferred.",
            "An exact source quotation does not establish the semantic truth of a statement.",
            "Assembled parameter fields are unverified lexical observations, never validated definitions.",
            "Unassembled semantic rows include continuations and malformed or bare declarations; they are not automatically missing parameters.",
            "Hypothetical semantic units lack a lexical entry anchor and are excluded from parameter assembly, verified coverage and message identity.",
            "Reconstructed table entries retain column fragments in row order; they do not resolve damaged cell boundaries, parameter syntax or message ownership.",
            "Table repair hypotheses retain possible bare type/name/unit tokens and raw column prefixes without assigning message identity or correcting the source.",
            "This model is not suitable for automatic code generation.",
        ],
    }
}

#[cfg(test)]
#[path = "../../tests/workflows/protocol_model_test.rs"]
mod tests;
