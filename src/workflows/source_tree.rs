// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Syntax-only source structure. No protocol names, message types or ownership
//! decisions belong here. Positions refer to physical source lines (zero-based).

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Section {
    pub heading_line: usize,
    pub level: usize,
    pub parent: Option<usize>,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Table {
    pub section: Option<usize>,
    pub start_line: usize,
    pub end_line: usize,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    pub line: usize,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Block {
    pub start_line: usize,
    pub end_line: usize,
    pub section: Option<usize>,
    pub kind: BlockKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockKind {
    Prose,
    FencedCode,
    Table(usize),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct SourceTree {
    pub sections: Vec<Section>,
    pub tables: Vec<Table>,
    pub blocks: Vec<Block>,
    pub section_at_line: Vec<Option<usize>>,
}

/// Source-backed structural context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructuralContext {
    pub section_path: Vec<usize>,
    pub block: Option<BlockKind>,
}

#[cfg(test)]
#[derive(Debug, Default)]
pub(super) struct SourceParser {
    input: simplelexer::tablelexer::DocumentInput,
}

#[cfg(test)]
impl SourceParser {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push_chunk(&mut self, chunk: &str) {
        self.push_bytes(chunk.as_bytes());
    }
    pub fn push_bytes(&mut self, chunk: &[u8]) {
        self.input.push_bytes(chunk);
    }
    pub fn finish(self) -> SourceTree {
        let source = self.input.finish().expect("source document must be UTF-8");
        SourceTree::from_analysis(&simplelexer::tablelexer::DocumentAnalysis::parse(&source))
    }
}

/// Observable syntax transition with an absolute, zero-based source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransitionKind {
    SectionStart,
    TableStart,
    TableEnd,
    FenceStart,
    FenceEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Transition {
    pub line: usize,
    pub kind: TransitionKind,
}

/// Structural edges are hypotheses about document layout, never entity ownership.
/// Both endpoints refer to zero-based physical source lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RelationKind {
    Contains,
    Continues,
    RefersTo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RelationEvidence {
    Syntax,
    ExplicitReference,
    ModelInference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HypothesisStatus {
    Observed,
    Proposed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructuralRelation {
    pub from_line: usize,
    pub to_line: usize,
    pub kind: RelationKind,
    pub evidence: RelationEvidence,
    pub status: HypothesisStatus,
    /// Exact source lines supporting this edge, not a model-generated quote.
    pub evidence_lines: Vec<usize>,
}

/// A proposed table-like region in text whose Markdown separators are missing.
/// No columns, message identities, or row ownership are invented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructureHypothesis {
    pub start_line: usize,
    pub end_line: usize,
    pub status: HypothesisStatus,
    pub evidence: RelationEvidence,
    pub relations: Vec<StructuralRelation>,
}

impl SourceTree {
    pub fn parse(document: &str) -> Self {
        Self::from_analysis(&simplelexer::tablelexer::DocumentAnalysis::parse(document))
    }

    /// Adapt lexer byte ranges to the physical line coordinates used by consumers.
    pub fn from_analysis(analysis: &simplelexer::tablelexer::DocumentAnalysis<'_>) -> Self {
        use simplelexer::tablelexer::RowKind;
        let mut starts = Vec::new();
        let mut offset = 0;
        for line in analysis.source.split_inclusive('\n') {
            starts.push(offset);
            offset += line.len();
        }
        let line_at = |byte: usize| {
            starts
                .partition_point(|&start| start <= byte)
                .saturating_sub(1)
        };
        let mut tree = Self::default();
        for (index, heading) in analysis.headings.iter().enumerate() {
            let path = analysis.heading_path_at(heading.span.start);
            let end_line = analysis.headings[index + 1..]
                .iter()
                .find(|next| next.level <= heading.level)
                .map_or(starts.len(), |next| line_at(next.span.start));
            tree.sections.push(Section {
                heading_line: line_at(heading.span.start),
                level: heading.level,
                parent: path.iter().rev().nth(1).copied(),
                end_line,
            });
        }
        for line in 0..starts.len() {
            let end = starts
                .get(line + 1)
                .copied()
                .unwrap_or(analysis.source.len());
            tree.section_at_line
                .push(analysis.heading_path_at(end - 1).last().copied());
        }
        for item in &analysis.tables {
            let table = analysis.table(item);
            let start_line = line_at(table.span.start);
            let end_line = line_at(table.span.end - 1) + 1;
            let section = item.heading_indices.last().copied();
            let index = tree.tables.len();
            tree.tables.push(Table {
                section,
                start_line,
                end_line,
                rows: table
                    .rows
                    .iter()
                    .filter(|row| row.kind != RowKind::Separator)
                    .map(|row| Row {
                        line: line_at(row.span.start),
                        cells: row
                            .cells
                            .iter()
                            .map(|cell| cell.text.trim().to_owned())
                            .collect(),
                    })
                    .collect(),
            });
            tree.blocks.push(Block {
                start_line,
                end_line,
                section,
                kind: BlockKind::Table(index),
            });
        }
        for span in &analysis.fences {
            let start_line = line_at(span.start);
            tree.blocks.push(Block {
                start_line,
                end_line: line_at(span.end - 1) + 1,
                section: tree.section_at_line[start_line],
                kind: BlockKind::FencedCode,
            });
        }
        for (line, text) in analysis.source.lines().enumerate() {
            if text.trim().is_empty()
                || tree.sections.iter().any(|s| s.heading_line == line)
                || tree
                    .blocks
                    .iter()
                    .any(|b| b.start_line <= line && line < b.end_line)
            {
                continue;
            }
            let section = tree.section_at_line[line];
            if let Some(block) = tree.blocks.last_mut()
                && block.kind == BlockKind::Prose
                && block.end_line == line
                && block.section == section
            {
                block.end_line += 1;
            } else {
                tree.blocks.push(Block {
                    start_line: line,
                    end_line: line + 1,
                    section,
                    kind: BlockKind::Prose,
                });
            }
        }
        tree.blocks.sort_by_key(|block| block.start_line);
        tree
    }

    /// Recover *candidate* table-shaped runs after lossy document conversion.
    /// This is a Markdown-layout hypothesis, not a Markdown parser extension:
    /// consecutive pipe-bearing lines in one section may be a damaged table.
    /// Isolated pipes, code fences and already parsed tables are excluded.
    pub fn layout_hypotheses(&self, document: &str) -> Vec<StructureHypothesis> {
        let mut hypotheses = Vec::new();
        let mut run: Vec<usize> = Vec::new();
        let mut previous_section = None;
        let mut flush = |run: &mut Vec<usize>| {
            if run.len() >= 2 {
                let start = run[0];
                let end = run[run.len() - 1] + 1;
                let relations = run
                    .windows(2)
                    .map(|pair| StructuralRelation {
                        from_line: pair[0],
                        to_line: pair[1],
                        kind: RelationKind::Continues,
                        evidence: RelationEvidence::Syntax,
                        status: HypothesisStatus::Proposed,
                        evidence_lines: pair.to_vec(),
                    })
                    .collect();
                hypotheses.push(StructureHypothesis {
                    start_line: start,
                    end_line: end,
                    status: HypothesisStatus::Proposed,
                    evidence: RelationEvidence::Syntax,
                    relations,
                });
            }
            run.clear();
        };
        for (line, source) in document.lines().enumerate() {
            let section = self.section_at_line.get(line).copied().flatten();
            let eligible = source.contains('|')
                && !matches!(
                    self.block_at_line(line).map(|block| block.kind),
                    Some(BlockKind::FencedCode | BlockKind::Table(_))
                );
            if !eligible || (!run.is_empty() && previous_section != section) {
                flush(&mut run);
            }
            if eligible {
                run.push(line);
            }
            previous_section = section;
        }
        flush(&mut run);
        hypotheses
    }

    /// Events are source-positioned observations, not semantic ownership claims.
    pub fn transitions(&self) -> Vec<Transition> {
        let mut events = Vec::new();
        for section in &self.sections {
            events.push(Transition {
                line: section.heading_line,
                kind: TransitionKind::SectionStart,
            });
        }
        for table in &self.tables {
            events.push(Transition {
                line: table.start_line,
                kind: TransitionKind::TableStart,
            });
            events.push(Transition {
                line: table.end_line,
                kind: TransitionKind::TableEnd,
            });
        }
        for block in &self.blocks {
            if block.kind == BlockKind::FencedCode {
                events.push(Transition {
                    line: block.start_line,
                    kind: TransitionKind::FenceStart,
                });
                events.push(Transition {
                    line: block.end_line,
                    kind: TransitionKind::FenceEnd,
                });
            }
        }
        events.sort_by_key(|event| event.line);
        events
    }

    /// Retrieve structural scope at an absolute source line, independently
    /// of the extraction window used by an LLM.
    pub fn context_at_line(&self, line: usize) -> Option<StructuralContext> {
        let section = *self.section_at_line.get(line)?;
        let mut section_path = Vec::new();
        let mut current = section;
        while let Some(index) = current {
            section_path.push(index);
            current = self.sections[index].parent;
        }
        section_path.reverse();
        Some(StructuralContext {
            section_path,
            block: self.block_at_line(line).map(|block| block.kind),
        })
    }

    pub fn block_at_line(&self, line: usize) -> Option<&Block> {
        self.blocks
            .iter()
            .find(|block| block.start_line <= line && line < block.end_line)
    }
}

#[cfg(test)]
#[path = "../../tests/workflows/source_tree_test.rs"]
mod tests;
