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
    assembled_parameters: Vec<AssembledParameter>,
    unassembled_semantic_rows: Vec<UnassembledSemanticRow>,
    caveats: Vec<&'static str>,
}

/// Original-source inventories are independent of normalized message hypotheses.
/// Borrow the existing evidence rather than reparse or infer ownership.
#[derive(Serialize)]
pub(super) struct EntityModel<'a> {
    #[serde(flatten)]
    model: &'a ProtocolModel,
    source_entities: SourceEntities<'a>,
}

#[derive(Serialize)]
struct SourceEntities<'a> {
    coordinate_space: &'static str,
    assertion_semantics_verified: bool,
    parameters: &'a super::message_identity::ParameterResolution<'a>,
    enum_tables: &'a [super::message_identity::EnumTable],
    logical_enum_groups: &'a [super::message_identity::LogicalEnumGroup],
    logical_rows: &'a [super::message_identity::LogicalRowCandidate],
    html_grids: &'a [serde_json::Value],
    literal_enum_mappings: Vec<LiteralEnumMapping<'a>>,
    relationships: Vec<SourceRelationship>,
    relationship_review: RelationshipReview,
    entity_registry: EntityRegistry,
}

/// Literal table observations, independent of extracted assertions and ownership.
#[derive(Serialize)]
struct LiteralEnumMapping<'a> {
    member_id: String,
    table_header_offset: usize,
    row_start: usize,
    row_end: usize,
    value: &'a str,
    description: &'a str,
    duplicate_value: bool,
    conflicting_description: bool,
    status: &'static str,
}

fn literal_enum_mappings(
    tables: &[super::message_identity::EnumTable],
) -> Vec<LiteralEnumMapping<'_>> {
    tables.iter().flat_map(|table| {
        table.members.iter().map(move |member| LiteralEnumMapping {
            member_id: format!("enum_member:{}:{}", table.header_offset, member.row_start),
            table_header_offset: table.header_offset,
            row_start: member.row_start,
            row_end: member.row_end,
            value: &member.value,
            description: &member.description,
            duplicate_value: member.duplicate_value,
            conflicting_description: member.conflicting_description,
            status: "literal_source_mapping",
        })
    }).collect()
}

#[derive(Serialize)]
struct SourceRelationship {
    kind: &'static str,
    group_index: usize,
    table_header_offset: usize,
    member_row_start: Option<usize>,
    status: &'static str,
}

fn enum_relationships(
    tables: &[super::message_identity::EnumTable],
    groups: &[super::message_identity::LogicalEnumGroup],
) -> Vec<SourceRelationship> {
    let mut relations = Vec::new();
    for (group_index, group) in groups.iter().enumerate() {
        for &offset in &group.physical_header_offsets {
            let mut matches = tables.iter().filter(|table| table.header_offset == offset);
            let Some(table) = matches.next() else {
                continue;
            };
            if matches.next().is_some() {
                continue;
            }
            if groups
                .iter()
                .filter(|g| g.physical_header_offsets.contains(&offset))
                .count()
                != 1
                || group
                    .physical_header_offsets
                    .iter()
                    .filter(|&&o| o == offset)
                    .count()
                    != 1
            {
                continue;
            }
            relations.push(SourceRelationship {
                kind: "enum_group_contains_table",
                group_index,
                table_header_offset: offset,
                member_row_start: None,
                status: "structural_hypothesis",
            });
            for member in &table.members {
                relations.push(SourceRelationship {
                    kind: "enum_group_contains_member",
                    group_index,
                    table_header_offset: offset,
                    member_row_start: Some(member.row_start),
                    status: "structural_hypothesis",
                });
            }
        }
    }
    relations
}

#[derive(Serialize)]
struct ParameterRelationship {
    kind: &'static str,
    definition_offset: usize,
    target_index: usize,
    assertion_offset: Option<usize>,
    status: &'static str,
}

#[derive(Serialize)]
struct RelationshipIssue {
    kind: &'static str,
    source_index: usize,
    source_offset: Option<usize>,
    reason: &'static str,
}

#[derive(Serialize)]
struct ReferenceCandidate {
    message_index: usize,
    hint_index: usize,
    parameter_definition_offsets: Vec<usize>,
    status: &'static str,
}

#[derive(Serialize)]
struct RelationshipReview {
    parameter_relationships: Vec<ParameterRelationship>,
    message_parameter_candidates: Vec<ReferenceCandidate>,
    issues: Vec<RelationshipIssue>,
    message_enum_status: &'static str,
}

fn review_relationships(
    model: &ProtocolModel,
    parameters: &super::message_identity::ParameterResolution<'_>,
    tables: &[super::message_identity::EnumTable],
    groups: &[super::message_identity::LogicalEnumGroup],
) -> RelationshipReview {
    let mut review = RelationshipReview {
        parameter_relationships: Vec::new(),
        message_parameter_candidates: Vec::new(),
        issues: Vec::new(),
        message_enum_status: "requires_explicit_reference_evidence",
    };
    for (index, entity) in parameters.entities.iter().enumerate() {
        let offset = entity.definition.source_byte_offset;
        if parameters
            .entities
            .iter()
            .filter(|e| e.definition.source_byte_offset == offset)
            .count()
            != 1
        {
            review.issues.push(RelationshipIssue {
                kind: "parameter_definition",
                source_index: index,
                source_offset: Some(offset),
                reason: "duplicate_definition_anchor",
            });
            continue;
        }
        for (target_index, assertion) in entity.assertions.iter().enumerate() {
            review.parameter_relationships.push(ParameterRelationship {
                kind: "parameter_has_assertion",
                definition_offset: offset,
                target_index,
                assertion_offset: Some(assertion.source_byte_offset),
                status: "source_link_semantics_unverified",
            });
        }
        for (target_index, _) in entity.properties.iter().enumerate() {
            review.parameter_relationships.push(ParameterRelationship {
                kind: "parameter_has_literal_property",
                definition_offset: offset,
                target_index,
                assertion_offset: None,
                status: "literal_source_property",
            });
        }
    }
    for (index, assertion) in parameters.unresolved_assertions.iter().enumerate() {
        review.issues.push(RelationshipIssue {
            kind: "parameter_assertion",
            source_index: index,
            source_offset: Some(assertion.source_byte_offset),
            reason: "unresolved_assertion",
        });
    }
    for (message_index, message) in model.messages.iter().enumerate() {
        for (hint_index, hint) in message.parameter_hints.iter().enumerate() {
            let offsets: Vec<_> = parameters
                .entities
                .iter()
                .filter(|entity| entity.definition.parameter == hint.name)
                .map(|entity| entity.definition.source_byte_offset)
                .collect();
            let status = match offsets.len() {
                0 => "no_exact_name_candidate",
                1 => "name_match_only_not_ownership",
                _ => "ambiguous_name_candidates",
            };
            review
                .message_parameter_candidates
                .push(ReferenceCandidate {
                    message_index,
                    hint_index,
                    parameter_definition_offsets: offsets,
                    status,
                });
        }
    }
    for (index, group) in groups.iter().enumerate() {
        for &offset in &group.physical_header_offsets {
            let count = tables.iter().filter(|t| t.header_offset == offset).count();
            let reason = if count == 0 {
                "missing_table_anchor"
            } else if count > 1 {
                "duplicate_table_anchor"
            } else if groups
                .iter()
                .filter(|g| g.physical_header_offsets.contains(&offset))
                .count()
                != 1
                || group
                    .physical_header_offsets
                    .iter()
                    .filter(|&&o| o == offset)
                    .count()
                    != 1
            {
                "ambiguous_group_membership"
            } else {
                continue;
            };
            review.issues.push(RelationshipIssue {
                kind: "enum_group",
                source_index: index,
                source_offset: Some(offset),
                reason,
            });
        }
    }
    for (index, table) in tables.iter().enumerate() {
        for member in &table.members {
            if member.duplicate_value || member.conflicting_description {
                review.issues.push(RelationshipIssue {
                    kind: "enum_member",
                    source_index: index,
                    source_offset: Some(member.row_start),
                    reason: if member.conflicting_description {
                        "conflicting_description"
                    } else {
                        "duplicate_value"
                    },
                });
            }
        }
    }
    review
}

#[derive(Serialize)]
struct RegistryEntry {
    id: String,
    kind: &'static str,
    name: Option<String>,
    locator: serde_json::Value,
    status: &'static str,
}

#[derive(Serialize)]
struct EntityRegistry {
    schema_version: u32,
    identity_scope: &'static str,
    entries: Vec<RegistryEntry>,
    ambiguous_ids: Vec<String>,
}

fn entity_registry(
    model: &ProtocolModel,
    parameters: &super::message_identity::ParameterResolution<'_>,
    tables: &[super::message_identity::EnumTable],
    groups: &[super::message_identity::LogicalEnumGroup],
) -> EntityRegistry {
    use serde_json::json;
    let mut entries = Vec::new();
    for (index, message) in model.messages.iter().enumerate() {
        entries.push(RegistryEntry {
            id: format!("message_candidate:{}", message.candidate_id),
            kind: "message_candidate",
            name: Some(message.proposed_name.clone()),
            locator: json!({"message_index": index, "candidate_id": message.candidate_id}),
            status: message.identity_status,
        });
    }
    for (index, message) in model.messages.iter().enumerate() {
        if let Some(identifier) = message.declared_type.as_deref() {
            entries.push(RegistryEntry {
                id: format!("message_identifier:{}", message.candidate_id),
                kind: "message_identifier",
                name: Some(identifier.to_owned()),
                locator: json!({"message_index": index, "candidate_id": message.candidate_id,
                    "declaration_assertions": message.assertions.iter().enumerate()
                        .filter(|(_, assertion)| assertion.evidence_quote.as_deref()
                            .and_then(|quote| declared_identifier(quote)) == Some(identifier))
                        .map(|(index, _)| index).collect::<Vec<_>>()
                }),
                status: "explicit_type_hypothesis",
            });
        }
        for (hint_index, hint) in message.parameter_hints.iter().enumerate() {
            entries.push(RegistryEntry {
                id: format!("parameter_candidate:{}:{hint_index}", message.candidate_id),
                kind: "parameter_candidate",
                name: Some(hint.name.clone()),
                locator: json!({"message_index": index, "candidate_id": message.candidate_id,
                    "hint_index": hint_index, "origin_assertion": hint.origin_assertion}),
                status: hint.syntax_kind,
            });
        }
    }
    for (index, entity) in parameters.entities.iter().enumerate() {
        let offset = entity.definition.source_byte_offset;
        entries.push(RegistryEntry {
            id: format!("parameter:{offset}"),
            kind: "parameter_definition",
            name: Some(entity.definition.parameter.to_string()),
            locator: json!({"definition_index": index, "original_byte_offset": offset}),
            status: "source_definition_semantics_unverified",
        });
    }
    for (index, table) in tables.iter().enumerate() {
        let offset = table.header_offset;
        entries.push(RegistryEntry {
            id: format!("enum_table:{offset}"),
            kind: "enum_table",
            name: None,
            locator: json!({"table_index": index, "original_byte_offset": offset}),
            status: "source_table",
        });
        for (member_index, member) in table.members.iter().enumerate() {
            entries.push(RegistryEntry {
                id: format!("enum_member:{offset}:{}", member.row_start),
                kind: "enum_member",
                name: Some(member.value.to_string()),
                locator: json!({"table_index": index, "member_index": member_index,
                    "original_byte_offset": member.row_start, "table_header_offset": offset}),
                status: "source_member_semantics_unverified",
            });
        }
    }
    for (index, group) in groups.iter().enumerate() {
        let mut offsets = group.physical_header_offsets.clone();
        offsets.sort_unstable();
        entries.push(RegistryEntry {
            id: format!(
                "enum_group:{}",
                offsets
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(":")
            ),
            kind: "enum_group",
            name: group.caption.clone(),
            locator: json!({"group_index": index, "physical_header_offsets": offsets}),
            status: "structural_hypothesis",
        });
    }
    let mut counts = std::collections::BTreeMap::new();
    for entry in &entries {
        *counts.entry(entry.id.clone()).or_insert(0usize) += 1;
    }
    let ambiguous_ids = counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id))
        .collect();
    EntityRegistry {
        schema_version: 1,
        identity_scope: "single_document_snapshot",
        entries,
        ambiguous_ids,
    }
}

impl ProtocolModel {
    pub(super) fn with_source_entities<'a>(
        &'a self,
        parameters: &'a super::message_identity::ParameterResolution<'a>,
        enum_tables: &'a [super::message_identity::EnumTable],
        logical_enum_groups: &'a [super::message_identity::LogicalEnumGroup],
        logical_rows: &'a [super::message_identity::LogicalRowCandidate],
        html_grids: &'a [serde_json::Value],
    ) -> EntityModel<'a> {
        EntityModel {
            model: self,
            source_entities: SourceEntities {
                coordinate_space: "original_source_utf8_bytes",
                assertion_semantics_verified: false,
                parameters,
                enum_tables,
                logical_enum_groups,
                logical_rows,
                html_grids,
                literal_enum_mappings: literal_enum_mappings(enum_tables),
                relationships: enum_relationships(enum_tables, logical_enum_groups),
                relationship_review: review_relationships(
                    self,
                    parameters,
                    enum_tables,
                    logical_enum_groups,
                ),
                entity_registry: entity_registry(
                    self,
                    parameters,
                    enum_tables,
                    logical_enum_groups,
                ),
            },
        }
    }
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
        self.hypothetical_semantic_units = project_semantic_units(
            graph,
            hypothetical,
            "hypothetical_voted_boundary_without_lexical_anchor",
        );

        self.reconstructed_table_entries = reconstruct_table_entries(&self.semantic_units);
        self.unassembled_semantic_rows = self
            .semantic_units
            .iter()
            .flat_map(|unit| &unit.rows)
            .filter(|row| {
                !row.cells
                    .iter()
                    .any(|cell| declared_parameter(cell).is_some())
            })
            .map(|row| UnassembledSemanticRow {
                normalized_byte_start: row.normalized_byte_start,
                cells: row.cells.clone(),
                reason: "no_explicit_angle_bracket_declaration",
                message_owner: None,
            })
            .collect();
        self.assembled_parameters = self.semantic_units.iter().flat_map(assemble_unit).collect();
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
    units
        .iter()
        .filter_map(|unit| {
            let rows: Option<Vec<_>> = unit
                .row_starts
                .iter()
                .map(|start| {
                    graph
                        .rows
                        .iter()
                        .find(|row| row.start == *start)
                        .map(|row| TableRowEvidence {
                            normalized_byte_start: row.start,
                            normalized_byte_end: None,
                            cells: row.cells.clone(),
                        })
                })
                .collect();
            let relations: Option<Vec<_>> = unit
                .supporting_relations
                .iter()
                .map(|index| {
                    graph.relations.get(*index).map(|r| RelationEvidence {
                        earlier_normalized_byte: r.earlier_row,
                        later_normalized_byte: r.later_row,
                        relation: r.relation.clone(),
                        votes: r.votes,
                        valid_attempts: r.valid_attempts,
                    })
                })
                .collect();
            Some(SemanticUnitEvidence {
                entry_normalized_byte: unit.entry_row,
                row_normalized_bytes: unit.row_starts.clone(),
                rows: rows?,
                supporting_relations: relations?,
                status,
            })
        })
        .collect()
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
    units
        .iter()
        .filter(|unit| unit.rows.len() > 1)
        .map(|unit| ReconstructedTableEntry {
            entry_normalized_byte: unit.entry_normalized_byte,
            row_normalized_bytes: unit.row_normalized_bytes.clone(),
            first_column_fragments: unit
                .rows
                .iter()
                .map(|row| row.cells.first().cloned().unwrap_or_default())
                .collect(),
            second_column_fragments: unit
                .rows
                .iter()
                .map(|row| row.cells.get(1).cloned().unwrap_or_default())
                .collect(),
            supporting_relations: unit.supporting_relations.clone(),
            status: "voted_row_sequence_uninterpreted",
            message_owner: None,
        })
        .collect()
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
        let token =
            token.trim_matches(|c: char| matches!(c, '|' | '`' | ',' | ';' | '*' | '(' | ')'));
        let Some((name, value)) = token.split_once('=') else {
            continue;
        };
        if name.is_empty()
            || !name.starts_with(|c: char| c.is_ascii_alphabetic())
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }
        // Read the complete declaration from the original cell: its payload may
        // contain spaces (e.g. "int 0 to 65535").
        let Some(token_start) = cell.find(token) else {
            continue;
        };
        let full_value = &cell[token_start + token.len() - value.len()..];
        let Some(rest) = full_value
            .strip_prefix('<')
            .or_else(|| full_value.strip_prefix("&lt;"))
        else {
            continue;
        };
        let payload = if let Some(end) = rest.find('>') {
            &rest[..end]
        } else {
            rest
        };
        let ty = payload.split_whitespace().next().unwrap_or("");
        if ty.is_empty() || !ty.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let tail = payload[ty.len()..].trim();
        let (range, unit) = if let Some((lo, hi)) = tail.split_once(" to ") {
            if !lo.is_empty()
                && !hi.is_empty()
                && lo.chars().all(|c| c.is_ascii_digit())
                && hi.chars().all(|c| c.is_ascii_digit())
            {
                (Some(tail.to_owned()), None)
            } else {
                (None, None)
            }
        } else if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_alphabetic() || c == '_') {
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
    declared_type: Option<String>,
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
                declared_type: unique_declared_identifier(
                    assertions
                        .iter()
                        .filter_map(|a| a.evidence_quote.as_deref()),
                ),
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
            "This model is not suitable for automatic code generation.",
        ],
    }
}

#[cfg(test)]
#[path = "../../tests/workflows/protocol_model_test.rs"]
mod tests;

pub(super) fn reference_candidates(
    model: &ProtocolModel,
    parameters: &super::message_identity::ParameterResolution<'_>,
    tables: &[super::message_identity::EnumTable],
    groups: &[super::message_identity::LogicalEnumGroup],
    normalized: &serde_json::Value,
    reviews: &[serde_json::Value],
    source: (
        &super::message_identity::OriginalTableRoleStats,
        &simplelexer::tablelexer::DocumentAnalysis<'_>,
    ),
) -> serde_json::Value {
    let registry = entity_registry(model, parameters, tables, groups);
    let mut report = reference_candidates_from_registry(&registry, normalized);
    annotate_reference_context(&mut report, &registry, model, normalized, reviews);
    annotate_structural_context(&mut report, &registry, normalized, reviews, source);
    classify_reference_gaps(&mut report, &registry);
    report["registry_audit"] = registry_name_audit(model, &registry);
    report
}

fn reference_candidates_from_registry(
    registry: &EntityRegistry,
    normalized: &serde_json::Value,
) -> serde_json::Value {
    use serde_json::json;
    let mut references = Vec::new();
    let mut counts = std::collections::BTreeMap::new();
    if let Some(groups) = normalized["groups"].as_array() {
        for (index, group) in groups.iter().enumerate() {
            let hypothesis = &group["hypothesis"];
            let fields: &[(&str, &str)] = match group["kind"].as_str() {
                Some("relations") => &[("subject", "subject_kind"), ("object", "object_kind")],
                Some("behaviors") => &[("actor", ""), ("target", "")],
                Some("instances") => &[("entity", ""), ("property", "")],
                _ => &[],
            };
            for &(field, kind_field) in fields {
                let Some(name) = hypothesis[field].as_str() else {
                    continue;
                };
                let kind = hypothesis[kind_field].as_str().unwrap_or("untyped");
                let allowed: &[&str] = match kind {
                    "message" => &["message_identifier"],
                    "parameter" => &["parameter_definition", "parameter_candidate"],
                    "entity" | "untyped" => &[
                        "message_identifier",
                        "parameter_definition",
                        "parameter_candidate",
                        "enum_group",
                    ],
                    _ => &[],
                };
                let literal = matches!(
                    kind,
                    "data_type"
                        | "identifier"
                        | "value"
                        | "unit"
                        | "range"
                        | "count"
                        | "boolean"
                        | "text"
                        | "example"
                );
                let matches: Vec<_> = registry.entries.iter().enumerate()
                    .filter(|(_, entry)| allowed.contains(&entry.kind) && entry.name.as_deref() == Some(name.trim()))
                    .map(|(entry_index, entry)| json!({"id": entry.id, "entry_index": entry_index, "kind": entry.kind, "status": entry.status}))
                    .collect();
                let status = if literal {
                    "literal"
                } else if matches.is_empty() {
                    "unresolved"
                } else if matches.len() > 1
                    || registry
                        .ambiguous_ids
                        .iter()
                        .any(|id| matches[0]["id"].as_str() == Some(id.as_str()))
                {
                    "ambiguous"
                } else {
                    "unique_candidate"
                };
                *counts.entry(status).or_insert(0usize) += 1;
                references.push(json!({"group_index": index, "field": field, "name": name,
                    "term_kind": kind, "status": status, "candidates": matches}));
            }
        }
    }
    json!({"schema_version": 1, "diagnostic_only": true, "identity_verified": false,
        "counts": counts, "references": references})
}

fn registry_name_audit(model: &ProtocolModel, registry: &EntityRegistry) -> serde_json::Value {
    use serde_json::json;
    let messages: Vec<_> = model.messages.iter().map(|message| {
        let hints: Vec<_> = message.parameter_hints.iter().map(|hint| {
            let assertion = message.assertions.get(hint.origin_assertion);
            json!({
                "name": hint.name, "status": hint.status, "syntax_kind": hint.syntax_kind,
                "origin_assertion": hint.origin_assertion,
                "assertion": assertion,
                "definition_candidates": registry.entries.iter()
                    .filter(|entry| entry.kind == "parameter_definition" && entry.name.as_deref() == Some(hint.name.as_str()))
                    .map(|entry| &entry.id).collect::<Vec<_>>()
            })
        }).collect();
        json!({"candidate_id": message.candidate_id, "proposed_name": message.proposed_name,
            "identity_status": message.identity_status, "parameter_hints": hints})
    }).collect();
    let mut names = std::collections::BTreeMap::<(&str, &str), Vec<&str>>::new();
    for entry in &registry.entries {
        if let Some(name) = entry.name.as_deref() {
            names.entry((entry.kind, name)).or_default().push(&entry.id);
        }
    }
    let name_groups: Vec<_> = names
        .into_iter()
        .map(|((kind, name), ids)| json!({"kind": kind, "name": name, "ids": ids}))
        .collect();
    json!({"diagnostic_only": true, "messages": messages, "name_groups": name_groups})
}

fn annotate_reference_context(
    report: &mut serde_json::Value,
    registry: &EntityRegistry,
    model: &ProtocolModel,
    normalized: &serde_json::Value,
    reviews: &[serde_json::Value],
) {
    use serde_json::json;
    let mut counts = std::collections::BTreeMap::new();
    if let Some(references) = report["references"].as_array_mut() {
        for reference in references {
            let task = reference["group_index"]
                .as_u64()
                .and_then(|i| normalized["groups"].get(i as usize))
                .and_then(|group| group["review_index"].as_u64())
                .and_then(|i| reviews.get(i as usize))
                .map(|review| &review["task"]);
            let mut matches = Vec::new();
            if let Some(task) = task {
                if let Some(candidates) = reference["candidates"].as_array_mut() {
                    for candidate in candidates {
                        let entry = candidate["entry_index"]
                            .as_u64()
                            .and_then(|i| registry.entries.get(i as usize));
                        let mut evidence = Vec::new();
                        if let Some(entry) = entry {
                            if let Some(message) = entry.locator["message_index"]
                                .as_u64()
                                .and_then(|i| model.messages.get(i as usize))
                                && task["candidate_id"].as_str()
                                    == Some(message.candidate_id.as_str())
                            {
                                evidence.push("same_extraction_candidate");
                                if entry.locator["origin_assertion"].as_u64().is_some()
                                    && entry.locator["origin_assertion"] == task["fact_index"]
                                {
                                    evidence.push("same_assertion");
                                }
                            }
                            if !evidence.is_empty() {
                                matches.push(entry.id.clone());
                            }
                        }
                        candidate["context_evidence"] = json!(evidence);
                    }
                }
                reference["origin"] = json!({
                    "candidate_id": task["candidate_id"],
                    "fact_index": task["fact_index"],
                    "window_index": task["window_index"],
                    "window_byte_offset": task["window_byte_offset"]
                });
            }
            let status = if task.is_none() {
                "missing_origin"
            } else if matches.is_empty() {
                "no_local_match"
            } else if matches.len() == 1 {
                "single_local_candidate"
            } else {
                "multiple_local_candidates"
            };
            *counts.entry(status).or_insert(0usize) += 1;
            reference["context_status"] = json!(status);
            reference["local_candidate_ids"] = json!(matches);
        }
    }
    report["context_counts"] = json!(counts);
    report["schema_version"] = json!(2);
}

fn annotate_structural_context(
    report: &mut serde_json::Value,
    registry: &EntityRegistry,
    normalized: &serde_json::Value,
    reviews: &[serde_json::Value],
    source: (
        &super::message_identity::OriginalTableRoleStats,
        &simplelexer::tablelexer::DocumentAnalysis<'_>,
    ),
) {
    use serde_json::json;
    let (roles, analysis) = source;
    let mut counts = std::collections::BTreeMap::new();
    if let Some(references) = report["references"].as_array_mut() {
        for reference in references {
            let context = reference["group_index"]
                .as_u64()
                .and_then(|i| normalized["groups"].get(i as usize))
                .and_then(|group| group["review_index"].as_u64())
                .and_then(|i| reviews.get(i as usize))
                .map(|review| &review["task"])
                .map(|task| {
                    if task["evidence_resolution"] == "unique_window_statement_match" {
                        &task["relocated_original_context"]
                    } else {
                        &task["original_context"]
                    }
                });
            let origin = context
                .and_then(|c| c["anchor_span"][0].as_u64())
                .map(|offset| offset as usize);
            if let Some(candidates) = reference["candidates"].as_array_mut() {
                for candidate in candidates {
                    let entry = candidate["entry_index"]
                        .as_u64()
                        .and_then(|i| registry.entries.get(i as usize));
                    let anchor = entry.and_then(|entry| match entry.kind {
                        "parameter_definition" => entry.locator["original_byte_offset"]
                            .as_u64()
                            .map(|offset| (offset as usize, "definition_row")),
                        "parameter_candidate" => {
                            let id = entry.locator["candidate_id"].as_str()?;
                            let index = entry.locator["origin_assertion"].as_u64()? as usize;
                            roles.assertion_spans.get(&(id.to_owned(), index)).map(
                                |&(start, _, exact)| {
                                    (
                                        start,
                                        if exact {
                                            "exact_assertion_anchor"
                                        } else {
                                            "structural_row"
                                        },
                                    )
                                },
                            )
                        }
                        _ => None,
                    });
                    let declaration_anchors: Vec<_> = entry
                        .filter(|entry| entry.kind == "message_identifier")
                        .and_then(|entry| {
                            Some((
                                entry.locator["candidate_id"].as_str()?,
                                entry.locator["declaration_assertions"].as_array()?,
                            ))
                        })
                        .into_iter()
                        .flat_map(|(id, indices)| {
                            indices.iter().filter_map(move |index| {
                                let index = index.as_u64()? as usize;
                                let &(start, end, exact) =
                                    roles.assertion_spans.get(&(id.to_owned(), index))?;
                                analysis.source.get(start..end)?;
                                Some((start, end, exact, index))
                            })
                        })
                        .collect();
                    let mut evidence = Vec::new();
                    let mut anchors = Vec::new();
                    for (start, end, exact, index) in declaration_anchors {
                        let mut local = Vec::new();
                        if let Some(left) = origin.filter(|&left| left < analysis.source.len()) {
                            if let (Some((lt, lr)), Some((rt, rr))) =
                                (analysis.source_row_at(left), analysis.source_row_at(start))
                                && lt.span == rt.span
                            {
                                local.push("same_original_table");
                                if lr.span == rr.span {
                                    local.push("same_original_row");
                                }
                            }
                            let headings = analysis.heading_path_at(left);
                            if !headings.is_empty() && headings == analysis.heading_path_at(start) {
                                local.push("same_original_section");
                            }
                        }
                        for item in &local {
                            if !evidence.contains(item) {
                                evidence.push(*item);
                            }
                        }
                        anchors.push(json!({"span": [start, end], "assertion_index": index,
                            "mapping": if exact { "exact_assertion_anchor" } else { "structural_row" },
                            "structural_evidence": local}));
                    }
                    candidate["declaration_anchors"] = json!(anchors);
                    if let (Some(left), Some((right, _))) = (origin, anchor)
                        && left < analysis.source.len()
                        && right < analysis.source.len()
                    {
                        if let (Some((lt, lr)), Some((rt, rr))) =
                            (analysis.source_row_at(left), analysis.source_row_at(right))
                            && lt.span == rt.span
                        {
                            evidence.push("same_original_table");
                            if lr.span == rr.span {
                                evidence.push("same_original_row");
                            }
                        }
                        let headings = analysis.heading_path_at(left);
                        if !headings.is_empty() && headings == analysis.heading_path_at(right) {
                            evidence.push("same_original_section");
                        }
                    }
                    for item in &evidence {
                        *counts.entry(*item).or_insert(0usize) += 1;
                    }
                    candidate["structural_evidence"] = json!(evidence);
                    candidate["original_anchor"] = json!(anchor.map(|(offset, mapping)| json!({
                        "byte_offset": offset, "mapping": mapping
                    })));
                }
            }
            reference["original_context_anchor"] = json!(origin);
            reference["origin_mapping"] = context
                .and_then(|c| c.get("mapping"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
        }
    }
    report["structural_evidence_counts"] = json!(counts);
    annotate_source_roles(report, analysis);
    annotate_entity_roles(report, analysis);
    report["schema_version"] = json!(6);
}

fn annotate_source_roles(
    report: &mut serde_json::Value,
    analysis: &simplelexer::tablelexer::DocumentAnalysis<'_>,
) {
    use serde_json::json;
    let Some(references) = report["references"].as_array_mut() else {
        return;
    };
    let mut offsets = Vec::new();
    for reference in references.iter() {
        offsets.extend(
            reference["original_context_anchor"]
                .as_u64()
                .map(|n| n as usize),
        );
        if let Some(candidates) = reference["candidates"].as_array() {
            for candidate in candidates {
                offsets.extend(
                    candidate["original_anchor"]["byte_offset"]
                        .as_u64()
                        .map(|n| n as usize),
                );
                if let Some(anchors) = candidate["declaration_anchors"].as_array() {
                    offsets.extend(
                        anchors
                            .iter()
                            .filter_map(|a| a["span"][0].as_u64().map(|n| n as usize)),
                    );
                }
            }
        }
    }
    offsets.sort_unstable();
    offsets.dedup();
    let roles = super::message_identity::original_context_roles(analysis, &offsets);
    let role_at = |value: &serde_json::Value| {
        value
            .as_u64()
            .and_then(|n| roles.get(&(n as usize)).copied())
            .unwrap_or("unknown")
    };
    let mut counts = std::collections::BTreeMap::new();
    for reference in references {
        let role = role_at(&reference["original_context_anchor"]);
        reference["origin_structural_role"] = json!(role);
        *counts.entry(role).or_insert(0usize) += 1;
        if let Some(candidates) = reference["candidates"].as_array_mut() {
            for candidate in candidates {
                let role = role_at(&candidate["original_anchor"]["byte_offset"]);
                candidate["anchor_structural_role"] = json!(role);
                if let Some(anchors) = candidate["declaration_anchors"].as_array_mut() {
                    for anchor in anchors {
                        let role = role_at(&anchor["span"][0]);
                        anchor["structural_role"] = json!(role);
                    }
                }
            }
        }
    }
    report["origin_structural_role_counts"] = json!(counts);
}

fn source_context_kind(
    analysis: &simplelexer::tablelexer::DocumentAnalysis<'_>,
    offset: &serde_json::Value,
    role: &serde_json::Value,
) -> &'static str {
    let Some(offset) = offset
        .as_u64()
        .map(|n| n as usize)
        .filter(|&n| n < analysis.source.len())
    else {
        return "unmapped";
    };
    if role == "example" {
        "example_context"
    } else if analysis.source_row_at(offset).is_some() {
        "table_row"
    } else {
        "other_source"
    }
}

fn candidate_entity_role(kind: &str, syntax: &str, context: &str) -> &'static str {
    if context == "example_context" {
        return "instance_or_usage_candidate";
    }
    match (kind, syntax) {
        ("parameter_definition", _) => "source_definition_candidate",
        ("message_identifier", _) => "identifier_declaration_hypothesis",
        ("parameter_candidate", "assignment_example_or_value") => "assignment_or_value_candidate",
        ("parameter_candidate", _) => "parameter_mention_candidate",
        _ => "unknown",
    }
}

fn annotate_entity_roles(
    report: &mut serde_json::Value,
    analysis: &simplelexer::tablelexer::DocumentAnalysis<'_>,
) {
    use serde_json::json;
    let Some(references) = report["references"].as_array_mut() else {
        return;
    };
    let mut counts = std::collections::BTreeMap::new();
    for reference in references {
        let context = source_context_kind(
            analysis,
            &reference["original_context_anchor"],
            &reference["origin_structural_role"],
        );
        reference["origin_context_kind"] = json!(context);
        reference["term_role"] = json!(if reference["status"] == "literal" {
            "literal"
        } else {
            "reference_candidate"
        });
        if let Some(candidates) = reference["candidates"].as_array_mut() {
            for candidate in candidates {
                let context = source_context_kind(
                    analysis,
                    &candidate["original_anchor"]["byte_offset"],
                    &candidate["anchor_structural_role"],
                );
                let role = candidate_entity_role(
                    candidate["kind"].as_str().unwrap_or(""),
                    candidate["status"].as_str().unwrap_or(""),
                    context,
                );
                candidate["anchor_context_kind"] = json!(context);
                candidate["entity_role"] = json!(role);
                *counts.entry(role).or_insert(0usize) += 1;
                if let Some(anchors) = candidate["declaration_anchors"].as_array_mut() {
                    for anchor in anchors {
                        let context = source_context_kind(
                            analysis,
                            &anchor["span"][0],
                            &anchor["structural_role"],
                        );
                        anchor["context_kind"] = json!(context);
                        anchor["entity_role"] =
                            json!(candidate_entity_role("message_identifier", "", context));
                    }
                }
            }
        }
    }
    report["candidate_entity_role_counts"] = json!(counts);
}

#[cfg(test)]
mod entity_role_tests {
    use super::candidate_entity_role;

    #[test]
    fn examples_do_not_become_definitions() {
        assert_eq!(
            candidate_entity_role("parameter_definition", "", "example_context"),
            "instance_or_usage_candidate"
        );
        assert_eq!(
            candidate_entity_role("parameter_definition", "", "table_row"),
            "source_definition_candidate"
        );
        assert_eq!(
            candidate_entity_role(
                "parameter_candidate",
                "assignment_example_or_value",
                "table_row"
            ),
            "assignment_or_value_candidate"
        );
        assert_eq!(
            candidate_entity_role("message_identifier", "", "unmapped"),
            "identifier_declaration_hypothesis"
        );
    }
}

/// Preserve normalized groups and all candidate evidence without resolving identities.
pub(super) fn referenced_hypotheses(
    normalized: &serde_json::Value,
    references: &serde_json::Value,
) -> serde_json::Value {
    use serde_json::json;
    let mut groups = Vec::new();
    let mut by_group = std::collections::BTreeMap::<usize, Vec<&serde_json::Value>>::new();
    let mut issues = Vec::new();
    let source_groups = normalized["groups"].as_array();
    let group_count = source_groups.map_or(0, Vec::len);
    if let Some(items) = references["references"].as_array() {
        for (index, reference) in items.iter().enumerate() {
            match reference["group_index"].as_u64().and_then(|n| usize::try_from(n).ok()) {
                Some(group) if group < group_count => by_group.entry(group).or_default().push(reference),
                _ => issues.push(json!({"reference_index": index, "reason": "invalid_group_index", "reference": reference})),
            }
        }
    }
    let mut counts = std::collections::BTreeMap::new();
    if let Some(items) = source_groups {
        for (index, group) in items.iter().enumerate() {
            let terms = by_group.remove(&index).unwrap_or_default();
            let status = if terms.is_empty() {
                "no_reference_terms"
            } else if terms.iter().any(|r| r["status"] == "unresolved") {
                "has_unresolved"
            } else if terms.iter().any(|r| r["status"] == "ambiguous") {
                "has_ambiguous"
            } else if terms.iter().all(|r| r["status"] == "literal") {
                "literal_only"
            } else if terms
                .iter()
                .all(|r| r["status"] == "literal" || r["status"] == "unique_candidate")
            {
                "unique_candidates_only"
            } else {
                "unknown"
            };
            *counts.entry(status).or_insert(0usize) += 1;
            groups.push(json!({"group_index": index, "status": status, "normalized_group": group, "references": terms}));
        }
    }
    json!({"schema_version": 1, "diagnostic_only": true, "identity_verified": false,
        "identity_scope": "single_document_snapshot", "group_count": group_count,
        "counts": counts, "groups": groups, "issues": issues,
        "potential_conflicts": normalized["potential_conflicts"]})
}

#[cfg(test)]
mod referenced_hypothesis_tests {
    use super::referenced_hypotheses;
    use serde_json::json;

    #[test]
    fn preserves_ambiguity_evidence_and_orphans() {
        let normalized = json!({"groups": [
            {"hypothesis": {"subject": "A"}, "occurrences": [{"evidence_quote": "A"}]},
            {"hypothesis": {"subject": "B"}}
        ], "potential_conflicts": []});
        let references = json!({"references": [
            {"group_index": 0, "status": "ambiguous", "candidates": [{"id": "a"}, {"id": "b"}]},
            {"group_index": 0, "status": "unresolved", "candidates": []},
            {"group_index": 9, "status": "unique_candidate"}
        ]});
        let report = referenced_hypotheses(&normalized, &references);
        assert_eq!(
            report["groups"][0]["normalized_group"],
            normalized["groups"][0]
        );
        assert_eq!(
            report["groups"][0]["references"][0],
            references["references"][0]
        );
        assert_eq!(report["groups"][0]["status"], "has_unresolved");
        assert_eq!(report["groups"][1]["status"], "no_reference_terms");
        assert_eq!(
            report["issues"][0]["reference"],
            references["references"][2]
        );
        assert_eq!(report["identity_verified"], false);
    }
}

// Lexical declaration hypothesis, not proof of identity or ownership.
fn declared_identifier(source: &str) -> Option<&str> {
    let text = source
        .trim()
        .trim_start_matches('#')
        .trim()
        .trim_start_matches('|')
        .trim();
    let (label, rest) = text.split_once(':')?;
    if !["type", "identifier", "message", "command", "opcode"]
        .iter()
        .any(|name| label.trim().eq_ignore_ascii_case(name))
    {
        return None;
    }
    let token = rest
        .trim()
        .split(|c: char| c.is_whitespace() || c == '|')
        .next()?;
    let token = token.trim_matches('`');
    (!token.is_empty()
        && token
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.')))
    .then_some(token)
}

fn unique_declared_identifier<'a>(quotes: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut identifiers = quotes.filter_map(declared_identifier);
    let first = identifiers.next()?;
    identifiers
        .all(|other| other == first)
        .then(|| first.to_owned())
}

#[cfg(test)]
mod generic_identifier_tests {
    use super::{declared_identifier, unique_declared_identifier};
    #[test]
    fn preserves_identifier_spelling() {
        for id in [
            "A",
            "a",
            "CONNECT_ACK",
            "0x1234",
            "device.reset",
            "GetConfiguration",
            "42",
        ] {
            assert_eq!(
                declared_identifier(&format!("Type: {id} | Description")),
                Some(id)
            );
        }
        assert_eq!(
            unique_declared_identifier(["Type: A", "Type: a"].into_iter()),
            None
        );
        assert_eq!(declared_identifier("Description: CONNECT"), None);
    }
}


/** Group matching candidate-bound relations without asserting entity identity.
 * Unresolved, ambiguous, behavioral and instance hypotheses remain separate.
 */
pub(super) fn candidate_equivalence_groups(referenced: &serde_json::Value) -> serde_json::Value {
    use serde_json::{Value, json};
    use std::collections::BTreeMap;
    let mut buckets: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    if let Some(groups) = referenced["groups"].as_array() {
        for (index, group) in groups.iter().enumerate() {
            let normalized = &group["normalized_group"];
            let mut hypothesis = normalized["hypothesis"].clone();
            let mut eligible = normalized["kind"] == "relations";
            let mut bindings = BTreeMap::new();
            if let Some(terms) = group["references"].as_array() {
                for term in terms {
                    let Some(field) = term["field"].as_str() else {
                        eligible = false;
                        continue;
                    };
                    if !matches!(field, "subject" | "object")
                        || bindings.contains_key(field)
                        || term["name"].as_str().is_none_or(|name| name.trim().is_empty())
                        || hypothesis[field].as_str() != term["name"].as_str()
                    {
                        eligible = false;
                        continue;
                    }
                    let binding = match term["status"].as_str() {
                        Some("literal") => json!(["literal", term["term_kind"], term["name"]]),
                        Some("unique_candidate") => {
                            match term["candidates"].as_array().map(Vec::as_slice) {
                                Some([candidate]) if candidate["id"].as_str().is_some()
                                    && candidate["entry_index"].as_u64().is_some() =>
                                {
                                    json!(["candidate", candidate["id"], candidate["entry_index"]])
                                }
                                _ => {
                                    eligible = false;
                                    continue;
                                }
                            }
                        }
                        _ => {
                            eligible = false;
                            continue;
                        }
                    };
                    bindings.insert(field, binding);
                }
            }
            eligible &= bindings.len() == 2
                && bindings.values().any(|b| b[0] == "candidate")
                && hypothesis.is_object();
            let key = if eligible {
                if let Some(fields) = hypothesis.as_object_mut() {
                    fields.remove("subject");
                    fields.remove("object");
                }
                json!(["candidate_equivalence", hypothesis, bindings]).to_string()
            } else {
                json!(["isolated", index]).to_string()
            };
            buckets.entry(key).or_default().push(group);
        }
    }
    let groups: Vec<Value> = buckets.into_iter().enumerate().map(|(index, (key, members))| {
        json!({"group_index": index, "candidate_equivalence": key.starts_with("[\"candidate_equivalence\""),
            "members": members})
    }).collect();
    // Only compare candidate-bound defaults with literal values and identical
    // remaining semantic fields. Different allowed values are not conflicts.
    let mut variants = BTreeMap::<String, Vec<usize>>::new();
    for (index, group) in groups.iter().enumerate() {
        if group["candidate_equivalence"] != true {
            continue;
        }
        let member = &group["members"][0];
        let mut hypothesis = member["normalized_group"]["hypothesis"].clone();
        if hypothesis["predicate"] != "has_default" {
            continue;
        }
        let Some(terms) = member["references"].as_array() else {
            continue;
        };
        let subject = terms.iter().find(|t| t["field"] == "subject");
        let object = terms.iter().find(|t| t["field"] == "object");
        let (Some(subject), Some(object)) = (subject, object) else {
            continue;
        };
        if subject["status"] != "unique_candidate" || object["status"] != "literal" {
            continue;
        }
        if let Some(fields) = hypothesis.as_object_mut() {
            fields.remove("subject");
            fields.remove("object");
        }
        let key = json!([hypothesis, subject["candidates"][0]["id"],
            subject["candidates"][0]["entry_index"], object["term_kind"]]).to_string();
        variants.entry(key).or_default().push(index);
    }
    let potential_conflicts: Vec<Value> = variants.into_values()
        .filter(|indices| indices.len() > 1)
        .map(|indices| json!({"kind": "candidate_default_variants", "group_indices": indices}))
        .collect();
    let input_count: usize = groups.iter().map(|g| g["members"].as_array().map_or(0, Vec::len)).sum();
    json!({"schema_version": 1, "diagnostic_only": true, "identity_verified": false,
        "input_group_count": input_count, "group_count": groups.len(),
        "groups": groups, "potential_conflicts": potential_conflicts, "source_issues": referenced["issues"],
        "source_potential_conflicts": referenced["potential_conflicts"]})
}


 // Classifications are observations about the registry, not semantic diagnoses.
fn classify_reference_gaps(report: &mut serde_json::Value, registry: &EntityRegistry) {
    use serde_json::json;
    let mut counts = std::collections::BTreeMap::new();
    if let Some(references) = report["references"].as_array_mut() {
        for reference in references {
            let name = reference["name"].as_str().unwrap_or("").trim();
            let reason = match reference["status"].as_str() {
                Some("unresolved") if registry.entries.iter().any(|entry|
                    entry.name.as_deref() == Some(name)) => "exact_name_in_other_registry_kind",
                Some("unresolved") if registry.entries.iter().any(|entry|
                    entry.name.as_deref().is_some_and(|label| label.eq_ignore_ascii_case(name))) =>
                    "case_variant_in_registry",
                Some("unresolved") => "no_exact_registry_name",
                Some("ambiguous") if reference["candidates"].as_array().is_some_and(|items|
                    items.iter().any(|item| registry.ambiguous_ids.iter().any(|id|
                        item["id"].as_str() == Some(id.as_str())))) => "ambiguous_registry_id",
                Some("ambiguous") => "multiple_candidates",
                _ => continue,
            };
            reference["gap_reason"] = json!(reason);
            *counts.entry(reason).or_insert(0usize) += 1;
        }
    }
    report["reference_gap_counts"] = json!(counts);
}
