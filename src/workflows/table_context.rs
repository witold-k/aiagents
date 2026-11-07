// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Source-grounded table metadata for extraction prompts. The lexer recognizes
//! explicit syntax only; flattened PDF text remains an unresolved hypothesis.
//! All offsets refer to the *same* normalized input used for extraction.
use simplelexer::tablelexer::{DocumentAnalysis, DocumentChunk, RowKind, TableType};

pub(super) fn table_context(analysis: &DocumentAnalysis<'_>, start: usize, end: usize) -> String {
    let mut out = String::new();
    let mut table_count = 0usize;
    let mut row_count = 0usize;
    for block in &analysis.blocks {
        let DocumentChunk::Table(table) = block else {
            continue;
        };
        if table.span.end <= start || table.span.start >= end {
            continue;
        }
        // Never pass cells from outside the source window as apparent evidence.
        let rows: Vec<_> = table
            .rows
            .iter()
            .filter(|row| {
                row.span.start >= start && row.span.end <= end && row.kind != RowKind::Separator
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        if table_count == 4 || row_count >= 24 {
            break;
        }
        table_count += 1;
        let kind = match table.table_type {
            TableType::Markdown => "Markdown",
            TableType::Html => "HTML",
        };
        let grid = table.audit_grid();
        out.push_str(&format!(
            "TABLE {}: syntax={} source_bytes={}..{} columns={} (explicit syntax only)\n",
            table_count, kind, table.span.start, table.span.end, grid.width
        ));
        for row in rows {
            if row_count == 24 {
                break;
            }
            row_count += 1;
            out.push_str(&format!(
                "  ROW source_bytes={}..{} kind={:?}",
                row.span.start, row.span.end, row.kind
            ));
            for cell in row.cells.iter().take(12) {
                let snippet: String = cell.text.chars().take(100).collect();
                let snippet = snippet.replace(['\n', '\r'], " ");
                out.push_str(&format!(
                    " | cell[{}] bytes={}..{} rowspan={} colspan={} text={:?}",
                    cell.column,
                    cell.span.start,
                    cell.span.end,
                    cell.rowspan,
                    cell.colspan,
                    snippet
                ));
            }
            out.push('\n');
        }
        if !grid.issues.is_empty() {
            out.push_str(&format!(
                "  GRID REVIEW: {} possible structural issues; do not infer missing cells\n",
                grid.issues.len()
            ));
        }
    }
    if out.is_empty() {
        return String::new();
    }
    format!(
        "=== EXPLICIT TABLE STRUCTURE (simplelexer, source-window-only) ===\n\
         The following metadata describes only syntax observed in the SOURCE window.\n\
         It is NOT a source quote or an identity declaration. Cite only SOURCE segment IDs.\n\
         Do not infer missing cells, row ownership, or message identity.\n{}\n",
        out
    )
}

#[cfg(test)]
#[path = "../../tests/workflows/table_context_test.rs"]
mod tests;
