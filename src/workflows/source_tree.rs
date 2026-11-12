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

fn heading_level(line: &str) -> Option<usize> {
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    ((1..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ')).then_some(level)
}

fn pipe_cells(line: &str) -> Option<Vec<String>> {
    let line = line.trim();
    if !line.contains('|') {
        return None;
    }
    let inner = line.strip_prefix('|').unwrap_or(line);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let cells: Vec<String> = inner
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect();
    (cells.len() >= 2).then_some(cells)
}

fn separator(cells: &[String]) -> bool {
    cells.iter().all(|cell| {
        let trimmed = cell.trim_matches(':');
        !trimmed.is_empty() && trimmed.bytes().all(|byte| byte == b'-')
    })
}

/// Source-backed structural context; deliberately contains no entity identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructuralContext {
    pub section_path: Vec<usize>,
    pub block: Option<BlockKind>,
}

/// Streaming syntax parser. Chunk boundaries have no structural meaning.
/// Only complete source lines are interpreted; the final line is flushed by finish().
#[derive(Debug, Default)]
pub(super) struct SourceParser {
    tree: SourceTree,
    sections: Vec<usize>,
    pending: Vec<u8>,
    pending_header: Option<(usize, Vec<String>)>,
    active_table: Option<usize>,
    active_prose: Option<usize>,
    fence: Option<(u8, usize, usize)>,
    line: usize,
}

impl SourceParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_chunk(&mut self, chunk: &str) {
        self.push_bytes(chunk.as_bytes());
    }

    /// Byte chunks may even split a UTF-8 code point. A complete line must
    /// still be valid UTF-8; invalid input is rejected rather than altered.
    pub fn push_bytes(&mut self, chunk: &[u8]) {
        self.pending.extend_from_slice(chunk);
        while let Some(end) = self.pending.iter().position(|&byte| byte == b'\n') {
            let bytes: Vec<u8> = self.pending.drain(..=end).collect();
            let line = std::str::from_utf8(&bytes[..bytes.len() - 1])
                .expect("source document must be UTF-8")
                .trim_end_matches('\r')
                .to_owned();
            self.push_line(&line);
        }
    }

    fn section(&self) -> Option<usize> {
        self.sections.last().copied()
    }

    fn close_prose(&mut self) {
        self.active_prose = None;
    }

    fn close_table(&mut self) {
        self.active_table = None;
    }

    fn add_prose(&mut self, line: usize) {
        if let Some(block) = self.active_prose {
            self.tree.blocks[block].end_line = line + 1;
        } else {
            let block = self.tree.blocks.len();
            self.tree.blocks.push(Block {
                start_line: line,
                end_line: line + 1,
                section: self.section(),
                kind: BlockKind::Prose,
            });
            self.active_prose = Some(block);
        }
    }

    fn start_table(&mut self, header_line: usize, header: Vec<String>) {
        self.close_prose();
        let section = self.section();
        let index = self.tree.tables.len();
        self.tree.tables.push(Table {
            section,
            start_line: header_line,
            end_line: self.line,
            rows: vec![Row {
                line: header_line,
                cells: header,
            }],
        });
        self.tree.blocks.push(Block {
            start_line: header_line,
            end_line: self.line,
            section,
            kind: BlockKind::Table(index),
        });
        self.active_table = Some(index);
    }

    fn extend_table(&mut self, cells: Vec<String>) {
        let index = self.active_table.expect("active table");
        self.tree.tables[index].end_line = self.line;
        self.tree.tables[index].rows.push(Row {
            line: self.line - 1,
            cells,
        });
        let block = self.tree.blocks.last_mut().expect("table block");
        debug_assert_eq!(block.kind, BlockKind::Table(index));
        block.end_line = self.line;
    }

    fn push_line(&mut self, source: &str) {
        let line = source.trim();
        let position = self.line;
        self.line += 1;
        // Fenced content is opaque, including headings and pipe characters.
        if let Some((marker, count, block)) = self.fence {
            let n = line.bytes().take_while(|&byte| byte == marker).count();
            self.tree.blocks[block].end_line = self.line;
            self.tree.section_at_line.push(self.section());
            if n >= count && line[n..].trim().is_empty() {
                self.fence = None;
            }
            return;
        }

        // A pending pipe row is only a table header when followed by a
        // Markdown separator of the same width.
        if let Some((header_line, header)) = self.pending_header.take() {
            if let Some(rule) = pipe_cells(line)
                && rule.len() == header.len()
                && separator(&rule)
            {
                self.start_table(header_line, header);
                self.tree.section_at_line.push(self.section());
                return;
            }
            self.add_prose(header_line);
        }

        if let Some(table) = self.active_table {
            if let Some(cells) = pipe_cells(line)
                && cells.len() == self.tree.tables[table].rows[0].cells.len()
                && !separator(&cells)
            {
                self.extend_table(cells);
                self.tree.section_at_line.push(self.section());
                return;
            }
            self.close_table();
        }

        if let Some(level) = heading_level(line) {
            self.close_prose();
            while self
                .sections
                .last()
                .is_some_and(|&s| self.tree.sections[s].level >= level)
            {
                let closed = self.sections.pop().expect("section stack");
                self.tree.sections[closed].end_line = position;
            }
            let section = self.tree.sections.len();
            self.tree.sections.push(Section {
                heading_line: position,
                level,
                parent: self.section(),
                end_line: self.line,
            });
            self.sections.push(section);
            self.tree.section_at_line.push(Some(section));
            return;
        }

        if line.starts_with("```") || line.starts_with("~~~") {
            self.close_prose();
            let marker = line.as_bytes()[0];
            let count = line.bytes().take_while(|&byte| byte == marker).count();
            let block = self.tree.blocks.len();
            self.tree.blocks.push(Block {
                start_line: position,
                end_line: self.line,
                section: self.section(),
                kind: BlockKind::FencedCode,
            });
            self.fence = Some((marker, count, block));
            self.tree.section_at_line.push(self.section());
            return;
        }

        if line.is_empty() {
            self.close_prose();
        } else if let Some(cells) = pipe_cells(line) {
            // Hold one potential header line until the following source line.
            self.pending_header = Some((position, cells));
        } else {
            self.add_prose(position);
        }
        self.tree.section_at_line.push(self.section());
    }

    pub fn finish(mut self) -> SourceTree {
        if !self.pending.is_empty() {
            let bytes = std::mem::take(&mut self.pending);
            let line = std::str::from_utf8(&bytes)
                .expect("source document must be UTF-8")
                .trim_end_matches('\r');
            self.push_line(line);
        }
        if let Some((position, _)) = self.pending_header.take() {
            self.add_prose(position);
        }
        for index in self.sections {
            self.tree.sections[index].end_line = self.line;
        }
        self.tree
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
        let mut parser = SourceParser::new();
        parser.push_chunk(document);
        parser.finish()
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

impl SourceTree {
    /// First-pass deterministic projection into source-spanned document scope.
    /// It makes no claims about message ownership or inferred semantics.
    pub(super) fn document_scope(
        &self,
        document: &str,
    ) -> crate::document_understanding::SemanticDocumentTree {
        use crate::document_understanding::{SemanticDocumentTree, SemanticNode};
        let mut offsets = vec![0usize];
        for (index, byte) in document.bytes().enumerate() {
            if byte == b'\n' {
                offsets.push(index + 1);
            }
        }
        offsets.push(document.len());
        let nodes = self
            .sections
            .iter()
            .map(|section| {
                let start = offsets[section.heading_line];
                let end = offsets[section.end_line];
                let heading_end = document[start..end]
                    .find('\n')
                    .map(|i| start + i)
                    .unwrap_or(end);
                let title = document[start..heading_end]
                    .trim()
                    .trim_start_matches('#')
                    .trim();
                SemanticNode {
                    title: title.to_owned(),
                    start_byte: start,
                    end_byte: end,
                    parent: section.parent,
                    context: Vec::new(),
                }
            })
            .collect();
        SemanticDocumentTree { nodes }
    }
}

#[cfg(test)]
#[path = "../../tests/workflows/source_tree_test.rs"]
mod tests;
