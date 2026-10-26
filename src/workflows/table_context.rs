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
mod tests {
    use super::*;

    #[test]
    fn context_contains_only_rows_inside_window() {
        let src = "Before\n| Name | Value |\n| --- | --- |\n| A | 1 |\n| B | 2 |\nAfter\n";
        let analysis = DocumentAnalysis::parse(src);
        let start = src.find("| A |").unwrap();
        let end = src.find("| B |").unwrap();
        let context = table_context(&analysis, start, end);
        assert!(context.contains("A"));
        assert!(context.contains("kind=Body"));
        assert!(!context.contains("text=\"B\""));
        assert!(table_context(&analysis, 0, 6).is_empty());
    }

    #[test]
    fn html_cells_keep_explicit_spans() {
        let src = concat!(
            "<table><tr><th>Type</th><th>Value</th></tr>",
            "<tr><td rowspan=\"2\">A</td><td>1</td></tr>",
            "<tr><td>2</td></tr></table>"
        );
        let analysis = DocumentAnalysis::parse(src);
        let context = table_context(&analysis, 0, src.len());
        assert!(context.contains("syntax=HTML"));
        assert!(context.contains("rowspan=2"));
        assert!(context.contains("columns=2"));
    }

    #[test]
    fn context_does_not_infer_flattened_tables() {
        let src = "name | value\nA | 1\nB | 2\n";
        let analysis = DocumentAnalysis::parse(src);
        assert!(table_context(&analysis, 0, src.len()).is_empty());
    }
}
