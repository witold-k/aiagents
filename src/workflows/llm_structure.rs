// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Read-only, bounded LLM hypotheses about adjacent lexer rows.
//! No message identity or parameter ownership is inferred here.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub(super) struct RowEvidence {
    pub line: usize,
    pub start: usize,
    pub previous: Option<usize>,
    pub next: Option<usize>,
    pub cells: Vec<String>,
}

/// Read-only source graph. A relation is a voted hypothesis, never ownership.
#[derive(Debug, Clone)]
pub(super) struct StructuralRelation {
    pub earlier_row: usize,
    pub later_row: usize,
    pub relation: String,
    pub votes: usize,
    pub valid_attempts: usize,
}

#[derive(Debug, Clone)]
pub(super) struct SemanticAssertion {
    pub source_line: usize,
    pub row_start: usize,
    pub lexer_kind: String,
    pub relations: Vec<usize>,
    // Deliberately absent: no message identity is derived from row adjacency.
}

#[derive(Debug, Clone)]
pub(super) struct SourceGraph {
    pub rows: Vec<RowEvidence>,
    pub relations: Vec<StructuralRelation>,
    pub assertions: Vec<SemanticAssertion>,
}

pub(super) fn source_graph(
    rows: &[RowEvidence],
    ranked: &[(usize, usize, String, usize)],
    valid_attempts: usize,
    uncovered: &[(usize, usize, String)],
) -> SourceGraph {
    let by_start: BTreeSet<_> = rows.iter().map(|row| row.start).collect();
    let relations: Vec<_> = ranked.iter()
        .filter(|(earlier, later, _, _)| by_start.contains(earlier) && by_start.contains(later))
        .map(|(earlier, later, relation, votes)| StructuralRelation {
            earlier_row: *earlier,
            later_row: *later,
            relation: relation.clone(),
            votes: *votes,
            valid_attempts,
        }).collect();
    let assertions = uncovered.iter()
        .map(|(line, row_start, kind)| SemanticAssertion {
            source_line: *line,
            row_start: *row_start,
            lexer_kind: kind.clone(),
            relations: relations.iter().enumerate()
                .filter_map(|(index, relation)| {
                    (relation.earlier_row == *row_start || relation.later_row == *row_start)
                        .then_some(index)
                }).collect(),
        }).collect();
    SourceGraph { rows: rows.to_vec(), relations, assertions }
}

/// Conservative read-only projection: only a directly evidenced entry plus its
/// description/continuations. A heading never becomes an entry's owner.
#[derive(Debug, Clone)]
pub(super) struct SemanticUnit {
    pub entry_row: usize,
    pub row_starts: Vec<usize>,
    pub supporting_relations: Vec<usize>,
}

/// Separate, read-only units whose entry boundary is supported by a voted
/// heading relation and a voted continuation, but has no lexical entry anchor.
/// They never participate in verified coverage or message identity.
pub(super) fn hypothetical_units(
    graph: &SourceGraph,
    verified: &[SemanticUnit],
) -> Vec<SemanticUnit> {
    let by_start: BTreeMap<_, _> = graph.rows.iter()
        .map(|row| (row.start, row)).collect();
    let occupied: BTreeSet<_> = verified.iter()
        .flat_map(|unit| unit.row_starts.iter().copied()).collect();
    let reserved: BTreeSet<_> = graph.assertions.iter()
        .filter(|assertion| assertion.lexer_kind == "reserved_message_type")
        .map(|assertion| assertion.row_start).collect();
    let majority = |earlier: usize, later: usize, kinds: &[&str]| -> Vec<usize> {
        let candidates: Vec<_> = graph.relations.iter().enumerate()
            .filter(|(_, relation)| relation.earlier_row == earlier
                && relation.later_row == later)
            .collect();
        let Some(valid) = candidates.first().map(|(_, relation)| relation.valid_attempts) else {
            return Vec::new();
        };
        if valid == 0 || candidates.iter().any(|(_, relation)| relation.valid_attempts != valid) {
            return Vec::new();
        }
        let support: Vec<_> = candidates.into_iter()
            .filter(|(_, relation)| kinds.contains(&relation.relation.as_str()))
            .collect();
        if support.iter().map(|(_, relation)| relation.votes).sum::<usize>() <= valid / 2 {
            return Vec::new();
        }
        support.into_iter().map(|(index, _)| index).collect()
    };
    let mut result = Vec::new();
    for (&entry, row) in &by_start {
        if entry_anchor(row) || occupied.contains(&entry) || reserved.contains(&entry) {
            continue;
        }
        let Some(previous) = row.previous else { continue };
        if by_start.get(&previous).and_then(|r| r.next) != Some(entry)
            || majority(previous, entry, &["heading_to_entry"]).is_empty() {
            continue;
        }
        let mut starts = vec![entry];
        let mut links = Vec::new();
        let mut cursor = entry;
        while let Some(next) = by_start.get(&cursor).and_then(|r| r.next) {
            if by_start.get(&next).and_then(|r| r.previous) != Some(cursor)
                || occupied.contains(&next) || reserved.contains(&next)
                || entry_anchor(by_start[&next]) || starts.contains(&next) {
                break;
            }
            let support = majority(cursor, next, &["entry_description", "text_continuation"]);
            if support.is_empty() { break; }
            links.extend(support);
            starts.push(next);
            cursor = next;
        }
        if starts.len() > 1 && !result.iter().any(|unit: &SemanticUnit| {
            unit.row_starts.iter().any(|start| starts.contains(start))
        }) {
            result.push(SemanticUnit {
                entry_row: entry, row_starts: starts, supporting_relations: links,
            });
        }
    }
    result
}

/// Count unresolved source-row occurrences covered by each proposed unit.
/// Exact row offsets only: this does not validate facts or assign message identity.
/// Duplicate occurrences remain distinct, and rows outside units remain uncovered.
pub(super) fn unit_fact_coverage(
    units: &[SemanticUnit],
    unmatched_rows: &[(usize, usize, String)],
) -> (Vec<(usize, usize)>, usize) {
    let mut counts = vec![0; units.len()];
    let mut outside = 0;
    for (_, row_start, _) in unmatched_rows {
        let matches: Vec<_> = units.iter().enumerate()
            .filter(|(_, unit)| unit.row_starts.contains(row_start))
            .map(|(index, _)| index)
            .collect();
        if matches.len() == 1 {
            counts[matches[0]] += 1;
        } else {
            // Ambiguous overlap is deliberately not counted as covered.
            outside += 1;
        }
    }
    (units.iter().zip(counts).map(|(unit, count)| (unit.entry_row, count)).collect(), outside)
}

/// Partition exact source-quote evidence by anchor strength. Each occurrence is
/// counted once; overlaps or missing/ambiguous units remain outside.
/// Evidence verifies only quote position within a lexer row, not statement truth.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct TieredEvidence {
    pub lexical_exact: usize,
    pub lexical_unverified: usize,
    pub relational_exact: usize,
    pub relational_unverified: usize,
    pub outside_or_ambiguous: usize,
}

pub(super) fn tiered_unit_evidence(
    lexical: &[SemanticUnit],
    relational: &[SemanticUnit],
    evidence: &[(usize, bool)],
) -> TieredEvidence {
    let mut result = TieredEvidence::default();
    for &(row_start, exact_quote) in evidence {
        let lexical_count = lexical.iter()
            .filter(|unit| unit.row_starts.contains(&row_start)).count();
        let relational_count = relational.iter()
            .filter(|unit| unit.row_starts.contains(&row_start)).count();
        match (lexical_count, relational_count, exact_quote) {
            (1, 0, true) => result.lexical_exact += 1,
            (1, 0, false) => result.lexical_unverified += 1,
            (0, 1, true) => result.relational_exact += 1,
            (0, 1, false) => result.relational_unverified += 1,
            _ => result.outside_or_ambiguous += 1,
        }
    }
    result
}

/// Classify only evidence outside both unit tiers. A heading is a structural
/// element only when its next row is a lexical entry and the link has majority
/// support. Reserved rows are explicit boundaries, not missing definitions.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct RemainingEvidence {
    pub structural_headings: usize,
    pub reserved_boundaries: usize,
    pub unresolved: usize,
}

pub(super) fn remaining_unit_evidence(
    graph: &SourceGraph,
    lexical: &[SemanticUnit],
    relational: &[SemanticUnit],
    unmatched: &[(usize, usize, String)],
) -> RemainingEvidence {
    let mut result = RemainingEvidence::default();
    for (_, start, kind) in unmatched {
        let matches = lexical.iter().chain(relational)
            .filter(|unit| unit.row_starts.contains(start)).count();
        if matches == 1 { continue; }
        // Ambiguous ownership remains unresolved, even for a reserved-looking row.
        if matches > 1 {
            result.unresolved += 1;
            continue;
        }
        if kind == "reserved_message_type" {
            result.reserved_boundaries += 1;
            continue;
        }
        let heading = graph.rows.iter().find(|row| row.start == *start)
            .and_then(|row| row.next)
            .and_then(|next| {
                let next_row = graph.rows.iter().find(|row| row.start == next)?;
                if next_row.previous != Some(*start) || !entry_anchor(next_row) {
                    return None;
                }
                let valid = graph.relations.iter().any(|relation| {
                    relation.earlier_row == *start
                        && relation.later_row == next
                        && relation.relation == "heading_to_entry"
                        && relation.valid_attempts > 0
                        && relation.votes > relation.valid_attempts / 2
                });
                Some(valid)
            }).unwrap_or(false);
        if heading {
            result.structural_headings += 1;
        } else {
            result.unresolved += 1;
        }
    }
    result
}

/// Explain why an unresolved source row is not covered by a semantic unit.
/// This is a read-only classification of evidence, not an inferred owner.
pub(super) fn outside_unit_evidence(
    graph: &SourceGraph,
    units: &[SemanticUnit],
    unmatched: &[(usize, usize, String)],
) -> Vec<(usize, usize, &'static str)> {
    let covered: BTreeSet<_> = units.iter()
        .flat_map(|unit| unit.row_starts.iter().copied())
        .collect();
    unmatched.iter().filter_map(|(line, start, kind)| {
        if covered.contains(start) { return None; }
        let row = graph.rows.iter().find(|row| row.start == *start)?;
        let boundary = kind == "reserved_message_type";
        let heading = graph.relations.iter().any(|relation| {
            relation.later_row == *start
                && relation.relation == "heading_to_entry"
                && relation.valid_attempts > 0
                && relation.votes > relation.valid_attempts / 2
        });
        let heading_to_next = graph.relations.iter().any(|relation| {
            relation.earlier_row == *start
                && relation.relation == "heading_to_entry"
                && relation.valid_attempts > 0
                && relation.votes > relation.valid_attempts / 2
        });
        let label = if boundary {
            "explicit_reserved_boundary"
        } else if entry_anchor(row) {
            "lexical_entry_without_unit"
        } else if heading {
            "heading_link_without_entry_anchor"
        } else if heading_to_next {
            "heading_before_entry"
        } else {
            "no_verified_unit_anchor"
        };
        Some((*line, *start, label))
    }).collect()
}

/// Recognize only a key explicitly present in both source cells.
/// This is an anchor, not a claim about message identity or ownership.
fn named_key_anchor(row: &RowEvidence) -> bool {
    let Some(name) = row.cells.first() else { return false };
    let Some((_, rest)) = name.rsplit_once('(') else { return false };
    let Some((key, _)) = rest.split_once(')') else { return false };
    !key.is_empty()
        && key.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && row.cells.get(1).is_some_and(|cell| {
            cell.trim_start().strip_prefix(key).is_some_and(|tail| tail.starts_with('='))
        })
}

/// A compact single-letter entry with a separate parameter-count/type cell.
/// The two cells provide independent syntax evidence; adjacency or LLM votes
/// alone never establish an entry boundary.
fn typed_entry_anchor(row: &RowEvidence) -> bool {
    let Some(first) = row.cells.first() else { return false };
    let mut words = first.split_whitespace();
    let Some(code) = words.next() else { return false };
    if code.len() != 1 || !code.as_bytes()[0].is_ascii_alphabetic() || words.next().is_none() {
        return false;
    }
    let Some(spec) = row.cells.get(1) else { return false };
    let mut fields = spec.split_whitespace();
    fields.next().is_some_and(|count| count.parse::<usize>().is_ok())
        && fields.next().is_some_and(|kind| {
            matches!(kind, "int" | "float" | "string" | "bool" | "boolean")
        })
}

fn entry_anchor(row: &RowEvidence) -> bool {
    named_key_anchor(row) || typed_entry_anchor(row)
}

/// Read-only units with explicit lexical anchors. A majority may disagree on
/// relation subtype but must agree that the next row belongs to the entry.
pub(super) fn semantic_units(graph: &SourceGraph) -> Vec<SemanticUnit> {
    let by_start: BTreeMap<_, _> = graph.rows.iter()
        .map(|row| (row.start, row)).collect();
    let mut units = Vec::new();
    for (&entry, row) in &by_start {
        if !entry_anchor(row) { continue; }
        let mut starts = vec![entry];
        let mut links = Vec::new();
        let mut cursor = entry;
        while let Some(next) = by_start.get(&cursor).and_then(|r| r.next) {
            if by_start.get(&next).and_then(|r| r.previous) != Some(cursor)
                || starts.contains(&next)
                || by_start.get(&next).is_some_and(|r| entry_anchor(r)) {
                break;
            }
            let candidates: Vec<_> = graph.relations.iter().enumerate()
                .filter(|(_, r)| r.earlier_row == cursor && r.later_row == next)
                .collect();
            let valid = candidates.first().map_or(0, |(_, r)| r.valid_attempts);
            if valid == 0 || candidates.iter().any(|(_, r)| r.valid_attempts != valid) {
                break;
            }
            let supporting: Vec<_> = candidates.into_iter()
                .filter(|(_, r)| matches!(r.relation.as_str(),
                    "entry_description" | "text_continuation"))
                .collect();
            let votes: usize = supporting.iter().map(|(_, r)| r.votes).sum();
            if votes <= valid / 2 { break; }
            links.extend(supporting.into_iter().map(|(index, _)| index));
            starts.push(next);
            cursor = next;
        }
        if starts.len() > 1 {
            units.push(SemanticUnit { entry_row: row.start, row_starts: starts, supporting_relations: links });
        }
    }
    units
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ValidRelation {
    earlier: usize,
    later: usize,
    relation: String,
}

fn allowed_pairs(rows: &[RowEvidence]) -> BTreeMap<(usize, usize), (&RowEvidence, &RowEvidence)> {
    let by_start: BTreeMap<_, _> = rows.iter().map(|row| (row.start, row)).collect();
    rows.iter().filter_map(|earlier| {
        let later = by_start.get(&earlier.next?)?;
        (later.previous == Some(earlier.start))
            .then_some(((earlier.start, later.start), (earlier, *later)))
    }).collect()
}

#[derive(Debug, Deserialize)]
struct Proposal {
    relations: Vec<serde_json::Value>,
}

struct Validation {
    accepted: BTreeSet<ValidRelation>,
    rejected: Vec<String>,
}

fn validate(proposal: &str, rows: &[RowEvidence]) -> Result<Validation, String> {
    let parsed: Proposal = serde_json::from_str(
        crate::utils::stringutils::strip_code_fences(proposal.trim()).trim(),
    ).map_err(|err| format!("invalid JSON: {err}"))?;
    let pairs = allowed_pairs(rows);
    if parsed.relations.len() > pairs.len().saturating_mul(4).max(1) {
        return Err("too many proposed relations".into());
    }
    let mut accepted = BTreeSet::new();
    let mut rejected = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, item) in parsed.relations.iter().enumerate() {
        let candidate = (|| {
            let earlier = item.get("earlier_row_byte").and_then(serde_json::Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or("invalid earlier_row_byte")?;
            let later = item.get("later_row_byte").and_then(serde_json::Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or("invalid later_row_byte")?;
            let relation = item.get("relation").and_then(serde_json::Value::as_str)
                .ok_or("missing relation")?;
            if !matches!(relation, "text_continuation" | "entry_description" | "heading_to_entry" | "independent" | "uncertain") {
                return Err("unknown relation label");
            }
            if !pairs.contains_key(&(earlier, later)) {
                return Err("pair is not in supplied evidence");
            }
            if !seen.insert((earlier, later)) {
                return Err("duplicate or conflicting pair");
            }
            Ok(ValidRelation { earlier, later, relation: relation.to_owned() })
        })();
        match candidate {
            Ok(relation) => { accepted.insert(relation); }
            Err(reason) => rejected.push(format!("item {}: {}", index + 1, reason)),
        }
    }
    Ok(Validation { accepted, rejected })
}

pub(super) fn prompt(rows: &[RowEvidence]) -> String {
    let evidence = allowed_pairs(rows).into_iter().map(|((earlier, later), (a, b))| {
        format!(
            "VALID PAIR earlier_row_byte={} later_row_byte={}\n  earlier_line={} earlier_cells={:?}\n  later_line={} later_cells={:?}",
            earlier, later, a.line, a.cells, b.line, b.cells
        )
    }).collect::<Vec<_>>().join("\n");
    format!(
        "Classify ONLY the VALID PAIR entries below. Copy both byte numbers EXACTLY from a VALID PAIR; never invent numbers or use null. \
         Each pair may appear at most once. You may omit pairs. \
         'text_continuation': the later row continues an unfinished sentence or text fragment from the earlier row; \
         'entry_description': the later row adds explanatory text to the same entry or parameter without continuing an unfinished sentence; \
         'heading_to_entry': earlier is a heading and later starts an entry; \
         'independent': later begins an independent entry; \
         'uncertain': evidence is insufficient to distinguish these relationships. \
         Do not infer message identity, parameter ownership, missing cells, or unseen text. \
         Return ONLY a JSON object with a relations array; each item has earlier_row_byte (integer), later_row_byte (integer), and relation (one of the five labels). \
         Do not use example byte numbers.\n\n{evidence}"
    )
}

pub(super) struct RankResult {
    pub valid_attempts: usize,
    pub ranked: Vec<(usize, usize, String, usize)>,
    pub rejected: Vec<(usize, String)>,
}

pub(super) fn rank(responses: &[String], rows: &[RowEvidence]) -> RankResult {
    let mut valid_attempts = 0;
    let mut rejected = Vec::new();
    let mut counts: BTreeMap<ValidRelation, usize> = BTreeMap::new();
    for (index, response) in responses.iter().enumerate() {
        match validate(response, rows) {
            Ok(validation) => {
                valid_attempts += 1;
                for relation in validation.accepted {
                    *counts.entry(relation).or_default() += 1;
                }
                for reason in validation.rejected {
                    rejected.push((index + 1, reason));
                }
            }
            Err(reason) => rejected.push((index + 1, reason)),
        }
    }
    let ranked = counts.into_iter().map(|(relation, votes)| {
        (relation.earlier, relation.later, relation.relation, votes)
    }).collect();
    RankResult { valid_attempts, ranked, rejected }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<RowEvidence> {
        vec![
            RowEvidence { line: 1, start: 10, previous: None, next: Some(20), cells: vec!["first".into()] },
            RowEvidence { line: 2, start: 20, previous: Some(10), next: None, cells: vec!["second".into()] },
            RowEvidence { line: 3, start: 30, previous: None, next: None, cells: vec!["other table".into()] },
        ]
    }

    fn graph_for_projection() -> SourceGraph {
        let starts = [10, 20, 30, 40, 50, 60, 70, 80];
        let cells = [
            vec!["in degrees.", ""],
            vec!["r Reference frequency required for BUC and LNB.", "2 int frequency MHz"],
            vec!["Frequency is in MHz.", "string Reference Used By: R, T,"],
            vec!["both (B)", ""],
            vec!["Time to Blockage (blockTime)", "blockTime=&lt;int sec>"],
            vec!["This parameter should accompany status message 18", ""],
            vec!["Additional Antenna Status (statusCode)", "statusCode=&lt;int 0 to 65535>"],
            vec!["If there are additional status messages", ""],
        ];
        let rows: Vec<_> = starts.iter().enumerate().map(|(i, start)| RowEvidence {
            line: i + 1,
            start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        }).collect();
        let ranked = vec![
            (10, 20, "entry_description".to_owned(), 3),
            (20, 30, "entry_description".to_owned(), 3),
            (30, 40, "text_continuation".to_owned(), 3),
            (40, 50, "independent".to_owned(), 3),
            (50, 60, "entry_description".to_owned(), 3),
            (60, 70, "independent".to_owned(), 3),
            (70, 80, "entry_description".to_owned(), 2),
            (70, 80, "text_continuation".to_owned(), 1),
        ];
        source_graph(&rows, &ranked, 3, &[])
    }

    #[test]
    fn semantic_projection_rejects_false_previous_entry_and_keeps_named_keys() {
        let graph = graph_for_projection();
        let units = semantic_units(&graph);
        assert_eq!(units.len(), 3);
        assert_eq!(units[0].entry_row, 20);
        assert_eq!(units[0].row_starts, vec![20, 30, 40]);
        assert_eq!(units[1].entry_row, 50);
        assert_eq!(units[1].row_starts, vec![50, 60]);
        assert_eq!(units[2].entry_row, 70);
        assert_eq!(units[2].row_starts, vec![70, 80]);
        assert!(units.iter().all(|u| !u.row_starts.contains(&10)));
    }

    #[test]
    fn unit_coverage_preserves_occurrences_and_excludes_unanchored_rows() {
        let units = semantic_units(&graph_for_projection());
        let unmatched = vec![
            (1, 10, "unclassified_row".to_owned()),
            (2, 20, "unclassified_row".to_owned()),
            (2, 20, "unclassified_row".to_owned()),
            (3, 30, "unclassified_row".to_owned()),
            (6, 60, "unclassified_row".to_owned()),
            (9, 90, "unclassified_row".to_owned()),
        ];
        let (counts, outside) = unit_fact_coverage(&units, &unmatched);
        assert_eq!(counts, vec![(20, 3), (50, 1), (70, 0)]);
        assert_eq!(outside, 2);
    }

    #[test]
    fn semantic_projection_does_not_cross_independent_boundary() {
        let graph = graph_for_projection();
        assert!(semantic_units(&graph).iter().all(|unit| {
            !unit.row_starts.windows(2).any(|pair| pair == [60, 70])
        }));
    }

    #[test]
    fn semantic_projection_keeps_full_description_chain_and_stops_at_next_key() {
        let starts = [110817, 110836, 110910, 110969, 111057, 114576, 114634, 114700, 114767, 114834, 114910, 114975];
        let cells = [
            vec!["in degrees.", ""],
            vec!["r Reference frequency required for BUC and LNB.", "2 int frequency MHz"],
            vec!["Frequency is in MHz.", "string Reference Used By: R, T,"],
            vec!["Reference required to be present on Rx (R), Tx (T) or", "or B"],
            vec!["both (B)", ""],
            vec!["Time to Blockage (blockTime)", "blockTime=&lt;int sec>"],
            vec!["This parameter should accompany status message 18", ""],
            vec!["of an impending blockage.", ""],
            vec!["seconds remaining before the blockage is expected to occur.", ""],
            vec!["Additional Antenna Status (statusCode)", "statusCode=&lt;int 0 to 65535>"],
            vec!["If there are additional status messages", ""],
            vec!["antenna, beyond that communicated", ""],
        ];
        let rows: Vec<_> = starts.iter().enumerate().map(|(i, start)| RowEvidence {
            line: i + 1,
            start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        }).collect();
        let ranked = vec![
            (110817, 110836, "entry_description".to_owned(), 3),
            (110836, 110910, "entry_description".to_owned(), 3),
            (110910, 110969, "text_continuation".to_owned(), 3),
            (110969, 111057, "text_continuation".to_owned(), 3),
            (114576, 114634, "entry_description".to_owned(), 3),
            (114634, 114700, "text_continuation".to_owned(), 3),
            (114700, 114767, "text_continuation".to_owned(), 3),
            (114767, 114834, "heading_to_entry".to_owned(), 3),
            (114834, 114910, "entry_description".to_owned(), 3),
            (114910, 114975, "text_continuation".to_owned(), 3),
        ];
        let graph = source_graph(&rows, &ranked, 3, &[]);
        let units = semantic_units(&graph);
        assert_eq!(units.len(), 3);
        assert_eq!(units[0].entry_row, 110836);
        assert_eq!(units[0].row_starts, vec![110836, 110910, 110969, 111057]);
        assert_eq!(units[1].entry_row, 114576);
        assert_eq!(units[1].row_starts, vec![114576, 114634, 114700, 114767]);
        assert_eq!(units[2].entry_row, 114834);
        assert_eq!(units[2].row_starts, vec![114834, 114910, 114975]);
        assert!(units.iter().all(|unit| !unit.row_starts.contains(&110817)));
    }

    #[test]
    fn outside_rows_keep_headings_separate_from_reserved_boundaries() {
        let starts = [10, 20, 30, 40];
        let cells = [
            vec!["Section title", ""],
            vec!["Continuation without an entry key", "int index"],
            vec!["(Optional key value pairs)", ""],
            vec!["q Reserved", ""],
        ];
        let rows: Vec<_> = starts.iter().enumerate().map(|(i, start)| RowEvidence {
            line: i + 1,
            start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        }).collect();
        let graph = source_graph(&rows, &[
            (10, 20, "heading_to_entry".to_owned(), 3),
            (20, 30, "independent".to_owned(), 3),
            (30, 40, "independent".to_owned(), 3),
        ], 3, &[]);
        let unmatched = vec![
            (2, 20, "unclassified_row".to_owned()),
            (3, 30, "possible_continuation".to_owned()),
            (4, 40, "reserved_message_type".to_owned()),
        ];
        assert_eq!(outside_unit_evidence(&graph, &[], &unmatched), vec![
            (2, 20, "heading_link_without_entry_anchor"),
            (3, 30, "no_verified_unit_anchor"),
            (4, 40, "explicit_reserved_boundary"),
        ]);
    }

    #[test]
    fn hypothetical_entry_requires_heading_and_continuation_without_ownership() {
        let starts = [10, 20, 30, 40, 50, 60];
        let cells = [
            vec!["Heading", ""],
            vec!["The next parameter describes an index", "int chain_index"],
            vec!["used for selecting the chain", ""],
            vec!["(Optional key value pairs)", ""],
            vec!["Named option (optionKey)", "optionKey=<int>"],
            vec!["q Reserved", ""],
        ];
        let rows: Vec<_> = starts.iter().enumerate().map(|(i, start)| RowEvidence {
            line: i + 1, start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        }).collect();
        let uncovered = vec![(6, 60, "reserved_message_type".to_owned())];
        let graph = source_graph(&rows, &[
            (10, 20, "heading_to_entry".to_owned(), 3),
            (20, 30, "text_continuation".to_owned(), 3),
            (30, 40, "independent".to_owned(), 3),
            (40, 50, "heading_to_entry".to_owned(), 3),
            (50, 60, "independent".to_owned(), 3),
        ], 3, &uncovered);
        let verified = semantic_units(&graph);
        let units = hypothetical_units(&graph, &verified);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].entry_row, 20);
        assert_eq!(units[0].row_starts, vec![20, 30]);
        assert!(!units[0].row_starts.contains(&40));
        assert!(!units[0].row_starts.contains(&60));
    }

    #[test]
    fn tiered_evidence_is_disjoint_and_preserves_duplicate_occurrences() {
        let lexical = vec![SemanticUnit {
            entry_row: 10, row_starts: vec![10, 20], supporting_relations: vec![],
        }];
        let relational = vec![SemanticUnit {
            entry_row: 30, row_starts: vec![30, 40], supporting_relations: vec![],
        }];
        let evidence = vec![
            (10, true), (10, true), (20, false),
            (30, true), (40, false), (50, true),
        ];
        assert_eq!(tiered_unit_evidence(&lexical, &relational, &evidence),
            TieredEvidence {
                lexical_exact: 2, lexical_unverified: 1,
                relational_exact: 1, relational_unverified: 1,
                outside_or_ambiguous: 1,
            });
        let overlapping = vec![SemanticUnit {
            entry_row: 30, row_starts: vec![10, 30], supporting_relations: vec![],
        }];
        assert_eq!(tiered_unit_evidence(&lexical, &overlapping, &[(10, true)]),
            TieredEvidence { outside_or_ambiguous: 1, ..TieredEvidence::default() });
    }

    #[test]
    fn remaining_evidence_distinguishes_headings_and_reserved_boundaries() {
        let starts = [10, 20, 30, 40];
        let cells = [
            vec!["(Optional key value pairs)", ""],
            vec!["Time to Blockage (blockTime)", "blockTime=<int sec>"],
            vec!["q Reserved", ""],
            vec!["Other text", ""],
        ];
        let rows: Vec<_> = starts.iter().enumerate().map(|(i, start)| RowEvidence {
            line: i + 1, start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        }).collect();
        let unmatched = vec![
            (1, 10, "possible_continuation".to_owned()),
            (3, 30, "reserved_message_type".to_owned()),
            (4, 40, "unclassified_row".to_owned()),
        ];
        let graph = source_graph(&rows,
            &[(10, 20, "heading_to_entry".to_owned(), 3)], 3, &unmatched);
        assert_eq!(remaining_unit_evidence(&graph, &[], &[], &unmatched),
            RemainingEvidence {
                structural_headings: 1, reserved_boundaries: 1, unresolved: 1,
            });
        let graph_without_votes = source_graph(&rows, &[], 3, &unmatched);
        assert_eq!(remaining_unit_evidence(&graph_without_votes, &[], &[], &unmatched),
            RemainingEvidence {
                structural_headings: 0, reserved_boundaries: 1, unresolved: 2,
            });
    }

    #[test]
    fn only_exact_same_table_adjacency_is_valid() {
        let good = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}]}"#;
        assert_eq!(validate(good, &rows()).unwrap().accepted.len(), 1);
        let bad = r#"{"relations":[{"earlier_row_byte":20,"later_row_byte":30,"relation":"text_continuation"}]}"#;
        assert_eq!(validate(bad, &rows()).unwrap().rejected.len(), 1);
    }

    #[test]
    fn fabricated_and_unknown_relations_are_rejected() {
        for response in [
            r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":999,"relation":"text_continuation"}]}"#,
            r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"message_identity"}]}"#,
        ] {
            assert_eq!(validate(response, &rows()).unwrap().rejected.len(), 1);
        }
    }

    #[test]
    fn heading_and_continuation_are_distinct() {
        let heading = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"heading_to_entry"}]}"#;
        let continuation = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}]}"#;
        assert_ne!(validate(heading, &rows()).unwrap().accepted, validate(continuation, &rows()).unwrap().accepted);
    }

    #[test]
    fn disagreement_is_retained_not_resolved_by_guessing() {
        let a = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}]}"#.to_owned();
        let b = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"independent"}]}"#.to_owned();
        let RankResult { valid_attempts: valid, ranked, rejected } = rank(&[a, b], &rows());
        assert!(rejected.is_empty());
        assert_eq!(valid, 2);
        assert_eq!(ranked.len(), 2);
        assert!(ranked.iter().all(|item| item.3 == 1));
    }
    #[test]
    fn invalid_items_do_not_discard_valid_pairs() {
        let response = r#"{"relations":[
            {"earlier_row_byte":10,"later_row_byte":20,"relation":"heading_to_entry"},
            {"earlier_row_byte":10,"later_row_byte":null,"relation":"independent"},
            {"earlier_row_byte":10,"later_row_byte":999,"relation":"independent"},
            {"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}
        ]}"#.to_owned();
        let result = rank(&[response], &rows());
        assert_eq!(result.valid_attempts, 1);
        assert_eq!(result.ranked.len(), 1);
        assert_eq!(result.ranked[0].2, "heading_to_entry");
        assert_eq!(result.rejected.len(), 3);
    }

    #[test]
    fn prompt_only_offers_real_pairs() {
        let prompt = prompt(&rows());
        assert!(prompt.contains("VALID PAIR earlier_row_byte=10 later_row_byte=20"));
        assert!(!prompt.contains("VALID PAIR earlier_row_byte=20 later_row_byte=30"));
    }

    #[test]
    fn entry_description_is_distinct_from_sentence_continuation() {
        let description = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"entry_description"}]}"#;
        let continuation = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}]}"#;
        assert_ne!(
            validate(description, &rows()).unwrap().accepted,
            validate(continuation, &rows()).unwrap().accepted
        );
        let prompt = prompt(&rows());
        assert!(prompt.contains("'entry_description'"));
        assert!(prompt.contains("unfinished sentence"));
    }

    #[test]
    fn uncertainty_is_valid_but_does_not_imply_ownership() {
        let uncertain = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"uncertain"}]}"#;
        let result = validate(uncertain, &rows()).unwrap();
        assert_eq!(result.accepted.len(), 1);
        assert!(result.rejected.is_empty());
    }

}
