// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Read-only LLM layout proposals. Source coordinates are authoritative;
//! the model may propose structure but never invent source or entity ownership.

use serde::Deserialize;

use super::source_tree::{BlockKind, SourceTree};

pub(super) const PROMPT: &str = r#"You reconstruct possible Markdown document layout from lossy plain text.
Return ONLY compact JSON: {"alternatives":[{"kind":"table_rows"|"prose"|"row_continuation"|"cell_continuation","start_line":INTEGER,"end_line":INTEGER,"evidence_lines":[INTEGER,...],"reason":"short"}]}.
Line numbers are absolute, zero-based, and end_line is exclusive.
Propose at least two competing interpretations when ambiguous, including prose.
Only refer to lines shown in the source. Treat HTML table tags as explicit syntax. Never infer message identity, meaning,
missing cells or absent text. Do not turn section headings or fenced code into rows.
The Markdown structure is a HYPOTHESIS, not an assumption or a fact."#;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Alternative {
    pub kind: AlternativeKind,
    pub start_line: usize,
    pub end_line: usize,
    pub evidence_lines: Vec<usize>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AlternativeKind {
    TableRows,
    Prose,
    RowContinuation,
    CellContinuation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelResponse {
    alternatives: Vec<Alternative>,
}

#[derive(Debug, Clone)]
pub(super) struct RankedAlternative {
    pub alternative: Alternative,
    pub evidence_count: usize,
    pub structural_score: i32,
}

/// Each candidate is checked against the exact region shown to the model.
/// Scoring measures independently verifiable evidence coverage, not truth.
/// Equal scores remain competing proposals; no winner is promoted to fact.
pub(super) fn evaluate(
    response: &str,
    source: &str,
    tree: &SourceTree,
    start: usize,
    end: usize,
) -> Result<Vec<RankedAlternative>, String> {
    let response = crate::utils::stringutils::strip_code_fences(response.trim());
    let parsed: ModelResponse =
        serde_json::from_str(&response).map_err(|error| format!("Invalid layout JSON: {error}"))?;
    let lines: Vec<&str> = source.lines().collect();
    let mut accepted = Vec::new();
    for candidate in parsed.alternatives {
        if candidate.start_line < start
            || candidate.end_line > end
            || candidate.start_line >= candidate.end_line
            || candidate.end_line > lines.len()
            || candidate.evidence_lines.is_empty()
            || candidate.reason.trim().is_empty()
            || candidate.evidence_lines.iter().any(|&line| {
                line < candidate.start_line
                    || line >= candidate.end_line
                    || lines[line].trim().is_empty()
                    || matches!(
                        tree.block_at_line(line).map(|block| block.kind),
                        Some(BlockKind::FencedCode | BlockKind::Table(_))
                    )
            })
            || candidate
                .evidence_lines
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            continue;
        }
        let section = tree.section_at_line[candidate.start_line];
        if (candidate.start_line..candidate.end_line)
            .any(|line| tree.section_at_line[line] != section)
        {
            continue;
        }
        let evidence_count = candidate.evidence_lines.len();
        let structural_score = structural_score(&candidate, &lines);
        accepted.push(RankedAlternative {
            alternative: candidate,
            evidence_count,
            structural_score,
        });
    }
    accepted.sort_by(|a, b| {
        b.structural_score
            .cmp(&a.structural_score)
            .then_with(|| b.evidence_count.cmp(&a.evidence_count))
            .then_with(|| a.alternative.start_line.cmp(&b.alternative.start_line))
            .then_with(|| a.alternative.end_line.cmp(&b.alternative.end_line))
    });
    Ok(accepted)
}

/// Bounded, overlapping source windows. Keep absolute coordinates; never
/// split a model window across a section boundary.
pub(super) fn analysis_windows(tree: &SourceTree, start: usize, end: usize) -> Vec<(usize, usize)> {
    const MAX_LINES: usize = 10;
    const OVERLAP: usize = 2;
    let mut result = Vec::new();
    let mut cursor = start;
    while cursor < end {
        let section = tree.section_at_line.get(cursor).copied().flatten();
        let section_end = (cursor + MAX_LINES).min(end);
        let mut stop = cursor + 1;
        while stop < section_end && tree.section_at_line.get(stop).copied().flatten() == section {
            stop += 1;
        }
        result.push((cursor, stop));
        if stop == end {
            break;
        }
        // Do not cross section boundaries while overlapping.
        cursor = if stop < end && tree.section_at_line.get(stop).copied().flatten() != section {
            stop
        } else {
            (stop - OVERLAP).max(cursor + 1)
        };
    }
    result
}

/// Explicit HTML tables are observed source syntax, not LLM hypotheses.
/// Return physical line spans, including multi-line HTML rows/cells.
pub(super) fn html_table_spans(source: &str, tree: &SourceTree) -> Vec<(usize, usize)> {
    let mut result = Vec::new();
    let mut active = None;
    for (line, text) in source.lines().enumerate() {
        if matches!(
            tree.block_at_line(line).map(|b| b.kind),
            Some(BlockKind::FencedCode)
        ) {
            continue;
        }
        let lower = text.to_ascii_lowercase();
        if active.is_none() && lower.contains("<table") {
            active = Some(line);
        }
        if lower.contains("</table>")
            && let Some(start) = active.take()
        {
            result.push((start, line + 1));
        }
    }
    result
}

/// Evidence-weighted heuristic, not a probability or semantic correctness score.
/// Explicit syntax and stable column counts are stronger than simply citing lines.
fn structural_score(candidate: &Alternative, lines: &[&str]) -> i32 {
    let cited: Vec<&str> = candidate
        .evidence_lines
        .iter()
        .map(|&n| lines[n].trim())
        .collect();
    let pipes: Vec<usize> = cited.iter().map(|line| line.matches('|').count()).collect();
    let pipe_rows = pipes.iter().filter(|&&n| n >= 2).count();
    let stable_columns = pipe_rows >= 2
        && pipes
            .iter()
            .filter(|&&n| n >= 2)
            .all(|&n| n == pipes.iter().copied().find(|&x| x >= 2).unwrap_or(0));
    let html = cited.iter().any(|line| {
        let l = line.to_ascii_lowercase();
        l.contains("<table") || l.contains("<tr") || l.contains("<td") || l.contains("<th")
    });
    let continuation = cited
        .iter()
        .any(|line| !line.contains('|') && !line.starts_with('#'));
    let mut score = (candidate.evidence_lines.len().min(4)) as i32;
    match candidate.kind {
        AlternativeKind::TableRows => {
            score += (pipe_rows.min(3) as i32) * 2;
            if stable_columns {
                score += 3;
            }
            if html {
                score += 4;
            }
            if pipe_rows == 0 && !html {
                score -= 3;
            }
        }
        AlternativeKind::RowContinuation | AlternativeKind::CellContinuation => {
            if continuation {
                score += 3;
            } else {
                score -= 2;
            }
            if pipe_rows > 0 || html {
                score += 2;
            }
        }
        AlternativeKind::Prose => {
            if pipe_rows == 0 && !html {
                score += 3;
            }
            if stable_columns || html {
                score -= 3;
            }
        }
    }
    score
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LineLayout {
    PipeRow,
    PossibleContinuation,
    Unresolved,
}

#[derive(Debug, Clone)]
pub(super) struct LineDecision {
    pub line: usize,
    pub layout: LineLayout,
    pub model_table_votes: usize,
    pub model_prose_votes: usize,
}

/// Merge overlapping model windows by absolute source line. Model votes are
/// diagnostic only: deterministic syntax takes precedence, and a disagreement
/// is never silently converted into a table or an ownership decision.
pub(super) fn reconcile(
    source: &str,
    tree: &SourceTree,
    regions: &[(usize, usize)],
    proposals: &[RankedAlternative],
) -> Vec<LineDecision> {
    use std::collections::BTreeSet;
    let lines: Vec<&str> = source.lines().collect();
    let covered: BTreeSet<usize> = regions
        .iter()
        .flat_map(|&(start, end)| start..end.min(lines.len()))
        .collect();
    covered
        .into_iter()
        .filter(|&line| {
            !matches!(
                tree.block_at_line(line).map(|block| block.kind),
                Some(BlockKind::FencedCode | BlockKind::Table(_))
            )
        })
        .map(|line| {
            let text = lines[line].trim();
            let pipes = text.matches('|').count();
            let model_table_votes = proposals
                .iter()
                .filter(|p| {
                    p.alternative.evidence_lines.contains(&line)
                        && p.alternative.kind == AlternativeKind::TableRows
                })
                .count();
            let model_prose_votes = proposals
                .iter()
                .filter(|p| {
                    p.alternative.evidence_lines.contains(&line)
                        && p.alternative.kind == AlternativeKind::Prose
                })
                .count();
            // Two delimiters define at least three cells even without outer
            // pipes (common in PDF-to-Markdown output). A single pipe is
            // ambiguous and stays a continuation candidate.
            let layout = if pipes >= 2 && !text.starts_with('#') {
                LineLayout::PipeRow
            } else if pipes > 0 && !text.starts_with('#') {
                LineLayout::PossibleContinuation
            } else {
                LineLayout::Unresolved
            };
            LineDecision {
                line,
                layout,
                model_table_votes,
                model_prose_votes,
            }
        })
        .collect()
}

/// Adjacent, equally wide pipe rows are evidence of a repeated column layout.
/// This does not attach one-pipe fragments to a row or infer cell ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RowAlignment {
    pub from_line: usize,
    pub to_line: usize,
    pub columns: usize,
}

/// Only compare rows in the same candidate region and section. The evidence
/// is local to the source, independent of LLM windows and their overlap.
pub(super) fn aligned_rows(
    source: &str,
    tree: &SourceTree,
    regions: &[(usize, usize)],
    decisions: &[LineDecision],
) -> Vec<RowAlignment> {
    use std::collections::BTreeSet;
    let lines: Vec<&str> = source.lines().collect();
    let rows: BTreeSet<usize> = decisions
        .iter()
        .filter(|decision| decision.layout == LineLayout::PipeRow)
        .map(|decision| decision.line)
        .collect();
    let mut edges = BTreeSet::new();
    for &(start, end) in regions {
        for line in start..end.min(lines.len()).saturating_sub(1) {
            let next = line + 1;
            if !rows.contains(&line) || !rows.contains(&next) {
                continue;
            }
            if tree.section_at_line[line] != tree.section_at_line[next] {
                continue;
            }
            let count = |index: usize| {
                let text = lines[index].trim();
                let pipes = text.matches('|').count();
                pipes + 1 - usize::from(text.starts_with('|')) - usize::from(text.ends_with('|'))
            };
            let columns = count(line);
            if columns >= 2 && columns == count(next) {
                edges.insert((line, next, columns));
            }
        }
    }
    edges
        .into_iter()
        .map(|(from_line, to_line, columns)| RowAlignment {
            from_line,
            to_line,
            columns,
        })
        .collect()
}

/// Local syntax context for a one-pipe fragment. A candidate is not an
/// attachment: even a row-shaped fragment may be ordinary prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FragmentShape {
    RowCandidate,
    CellContinuationCandidate,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FragmentContext {
    pub line: usize,
    pub previous_row: Option<usize>,
    pub next_row: Option<usize>,
    pub shape: FragmentShape,
}

/// Inspect only immediately adjacent physical lines in the same proposed
/// region and section. Do not bridge gaps or assign cell/message ownership.
pub(super) fn fragment_contexts(
    source: &str,
    tree: &SourceTree,
    regions: &[(usize, usize)],
    decisions: &[LineDecision],
) -> Vec<FragmentContext> {
    use std::collections::BTreeMap;
    let lines: Vec<&str> = source.lines().collect();
    let by_line: BTreeMap<usize, LineLayout> =
        decisions.iter().map(|d| (d.line, d.layout)).collect();
    decisions
        .iter()
        .filter(|d| d.layout == LineLayout::PossibleContinuation)
        .map(|d| {
            let line = d.line;
            let in_same_region = |other: usize| {
                regions.iter().any(|&(start, end)| {
                    start <= line && other >= start && line < end && other < end
                })
            };
            let is_row = |other: usize| {
                other < lines.len()
                    && in_same_region(other)
                    && tree.section_at_line.get(line) == tree.section_at_line.get(other)
                    && by_line.get(&other) == Some(&LineLayout::PipeRow)
            };
            let previous_row = line.checked_sub(1).filter(|&n| is_row(n));
            let next_row = line.checked_add(1).filter(|&n| is_row(n));
            let shape = match (previous_row, next_row) {
                (Some(before), Some(after)) => {
                    let columns = |index: usize| {
                        let text = lines[index].trim();
                        text.matches('|').count() + 1
                            - usize::from(text.starts_with('|'))
                            - usize::from(text.ends_with('|'))
                    };
                    if columns(before) == columns(after) {
                        FragmentShape::CellContinuationCandidate
                    } else {
                        FragmentShape::Ambiguous
                    }
                }
                (Some(_), None) => FragmentShape::CellContinuationCandidate,
                (None, Some(_)) => FragmentShape::RowCandidate,
                (None, None) => FragmentShape::Ambiguous,
            };
            FragmentContext {
                line,
                previous_row,
                next_row,
                shape,
            }
        })
        .collect()
}

pub(super) fn source_window(source: &str, start: usize, end: usize) -> String {
    source
        .lines()
        .enumerate()
        .skip(start)
        .take(end - start)
        .map(|(line, text)| format!("{line}: {text}\n"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_overlap_without_losing_absolute_lines() {
        let source = (0..27)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let tree = SourceTree::parse(&source);
        let windows = analysis_windows(&tree, 0, 27);
        assert_eq!(windows[0], (0, 10));
        assert_eq!(windows[1], (8, 18));
        assert_eq!(windows.last().copied(), Some((24, 27)));
    }

    #[test]
    fn html_table_spans_are_observed_outside_code() {
        let source = "# A\n<table>\n<tr><td>x</td></tr>\n</table>\n";
        let tree = SourceTree::parse(source);
        assert_eq!(html_table_spans(source, &tree), vec![(1, 4)]);
    }

    #[test]
    fn competing_proposals_remain_separate_and_ranked() {
        let source = "# Inventory\n| A | B |\n| C | D |\nEnd";
        let tree = SourceTree::parse(source);
        let response = r#"{"alternatives":[{"kind":"prose","start_line":1,"end_line":3,"evidence_lines":[1],"reason":"could be prose"},{"kind":"table_rows","start_line":1,"end_line":3,"evidence_lines":[1,2],"reason":"aligned rows"}]}"#;
        let ranked = evaluate(response, source, &tree, 1, 3).unwrap();
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].alternative.kind, AlternativeKind::TableRows);
        assert_eq!(ranked[0].evidence_count, 2);
        assert_eq!(ranked[1].alternative.kind, AlternativeKind::Prose);
    }

    #[test]
    fn pdf_table_rows_do_not_require_outer_pipes() {
        let source = "# Table\nType: P | Polarization | 2 | char\nword | description\nA sentence without delimiters";
        let tree = SourceTree::parse(source);
        let decisions = reconcile(source, &tree, &[(1, 4)], &[]);
        assert_eq!(decisions.len(), 3);
        assert_eq!(decisions[0].layout, LineLayout::PipeRow);
        assert_eq!(decisions[1].layout, LineLayout::PossibleContinuation);
        assert_eq!(decisions[2].layout, LineLayout::Unresolved);
    }

    #[test]
    fn reconciliation_deduplicates_overlaps_and_prefers_explicit_syntax() {
        let source = "# A\n| a | b |\nword | text\nordinary prose";
        let tree = SourceTree::parse(source);
        let proposals = evaluate(
            r#"{"alternatives":[{"kind":"prose","start_line":1,"end_line":3,"evidence_lines":[1,2],"reason":"ambiguous"}]}"#,
            source,
            &tree,
            1,
            3,
        )
        .unwrap();
        let decisions = reconcile(source, &tree, &[(1, 3), (2, 4)], &proposals);
        assert_eq!(decisions.len(), 3);
        assert_eq!(decisions[0].layout, LineLayout::PipeRow);
        assert_eq!(decisions[0].model_prose_votes, 1);
        assert_eq!(decisions[1].layout, LineLayout::PossibleContinuation);
        assert_eq!(decisions[2].layout, LineLayout::Unresolved);
    }

    #[test]
    fn aligned_rows_require_adjacent_matching_width_and_deduplicate_regions() {
        let source =
            "# A\na | b | c\nd | e | f\nfragment | text\ng | h | i\nj | k | l\n## B\nm | n | o";
        let tree = SourceTree::parse(source);
        let regions = [(1, 6), (2, 6), (5, 8)];
        let decisions = reconcile(source, &tree, &regions, &[]);
        let edges = aligned_rows(source, &tree, &regions, &decisions);
        assert_eq!(edges.len(), 2);
        assert_eq!(
            edges[0],
            RowAlignment {
                from_line: 1,
                to_line: 2,
                columns: 3
            }
        );
        assert_eq!(
            edges[1],
            RowAlignment {
                from_line: 4,
                to_line: 5,
                columns: 3
            }
        );
    }

    #[test]
    fn fragment_context_does_not_bridge_gaps_or_sections() {
        let source = "# A\na | b | c\nfragment | text\nd | e | f\nplain\norphan | text\n## B\nx | y | z\nnext | text";
        let tree = SourceTree::parse(source);
        let regions = [(1, 6), (7, 9)];
        let decisions = reconcile(source, &tree, &regions, &[]);
        let fragments = fragment_contexts(source, &tree, &regions, &decisions);
        assert_eq!(fragments.len(), 3);
        assert_eq!(fragments[0].shape, FragmentShape::CellContinuationCandidate);
        assert_eq!(fragments[0].previous_row, Some(1));
        assert_eq!(fragments[0].next_row, Some(3));
        assert_eq!(fragments[1].shape, FragmentShape::Ambiguous);
        assert_eq!(fragments[2].shape, FragmentShape::CellContinuationCandidate);
        assert_eq!(fragments[2].previous_row, Some(7));
        assert_eq!(fragments[2].next_row, None);
    }

    #[test]
    fn invented_locations_and_cross_section_proposals_are_rejected() {
        let source = "# First\nA | B\n## Second\nC | D";
        let tree = SourceTree::parse(source);
        let response = r#"{"alternatives":[{"kind":"table_rows","start_line":1,"end_line":4,"evidence_lines":[1,3],"reason":"invalid section"},{"kind":"prose","start_line":1,"end_line":2,"evidence_lines":[9],"reason":"invalid line"}]}"#;
        assert!(evaluate(response, source, &tree, 1, 4).unwrap().is_empty());
    }
}
