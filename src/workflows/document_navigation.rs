// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Read-only navigation region detection. Headings alone never suppress facts.
use super::source_tree::{BlockKind, SourceTree};
use serde_json::{Value, json};

fn kind(heading: &str) -> Option<&'static str> {
    let title = heading.trim_start_matches('#').trim().to_lowercase();
    let title = title.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ');
    match title {
        "contents" | "table of contents" | "inhaltsverzeichnis" => Some("table_of_contents"),
        "index" | "stichwortverzeichnis" | "sachregister" | "register" => Some("index"),
        "list of figures" | "abbildungsverzeichnis" => Some("list_of_figures"),
        "list of tables" | "tabellenverzeichnis" => Some("list_of_tables"),
        _ => None,
    }
}

fn reference_like(line: &str) -> bool {
    let s = line.trim().trim_start_matches("- ").trim();
    if s.is_empty() || s.starts_with('#') {
        return false;
    }
    if s.contains("](#") || s.contains("](http") {
        return true;
    }
    let last = s.split_whitespace().last().unwrap_or("");
    let token = last.trim_start_matches('.').trim_end_matches('.');
    let decimal = !token.is_empty() && token.chars().all(|c| c.is_ascii_digit());
    let roman = !token.is_empty()
        && token.len() <= 8
        && token.chars().all(|c| {
            matches!(
                c.to_ascii_lowercase(),
                'i' | 'v' | 'x' | 'l' | 'c' | 'd' | 'm'
            )
        });
    let dotted_number = s.rsplit_once("..").is_some_and(|(_, suffix)| {
        let suffix = suffix.trim_matches('.').trim();
        !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
    });
    let comma_pages = s.rsplit_once(',').is_some_and(|(_, tail)| {
        let tail = tail.trim();
        !tail.is_empty()
            && tail
                .chars()
                .all(|c| c.is_ascii_digit() || c == '-' || c == ' ')
    });
    ((decimal || roman) && s.len() > last.len() + 2) || dotted_number || comma_pages
}

fn reserved_declaration(line: &str) -> Option<(&'static str, &'static str)> {
    let content = line.trim().trim_start_matches('#').trim();
    let content = content.trim_start_matches('|').trim_end_matches('|').trim();
    let lower = content.to_ascii_lowercase();
    if lower.is_empty() || !lower.is_ascii() {
        return None;
    }
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    let reserved = tokens
        .iter()
        .position(|token| token.trim_matches(|c: char| !c.is_ascii_alphabetic()) == "reserved")?;
    // Descriptive uses and historical statements do not declare a current reservation.
    if lower.contains("reserved bandwidth")
        || lower.contains("reserved memory")
        || lower.contains("previously reserved")
        || lower.contains("formerly reserved")
        || lower.contains("no longer reserved")
        || lower.contains("not reserved")
        || lower.contains("reserved but now")
        || lower.contains("reserved, now")
    {
        return None;
    }
    let before = tokens[..reserved].join(" ");
    let after = tokens[reserved + 1..].join(" ");
    let future = after.starts_with("for future use")
        || after.starts_with("for future")
        || after.starts_with("for later use");
    let bare = before.is_empty() && (after.is_empty() || future);
    let label =
        before.trim_matches(|c: char| c == '|' || c == ':' || c == '-' || c.is_whitespace());
    let bits = label.starts_with("bit ")
        || label.starts_with("bits ")
        || label.starts_with("bit[")
        || label.starts_with("bits[");
    let numeric = label.starts_with("0x")
        || label.starts_with("0b")
        || label.chars().next().is_some_and(|c| c.is_ascii_digit());
    let identifiers = !label.is_empty()
        && label.len() <= 64
        && label.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, ' ' | '-' | '–' | ':' | '.' | '_' | '[' | ']' | '(' | ')')
        })
        && (label.contains(" - ")
            || label.contains(" to ")
            || label.contains("..")
            || label.len() <= 8);
    if bare {
        return Some(("unspecified", "unscoped"));
    }
    if bits {
        return Some(("bit_field", "explicit"));
    }
    if numeric {
        return Some(("value_range", "explicit"));
    }
    if identifiers {
        return Some(("identifier_or_range", "explicit"));
    }
    None
}

fn explicit_subject(raw: &str) -> &str {
    let value = raw.trim();
    if value
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("type:"))
    {
        value[5..].trim()
    } else {
        value
    }
}

fn pipe_cells(source: &str) -> Option<Vec<String>> {
    // A flattened or damaged table row can lack a Markdown separator.
    // Only inspect explicit pipe-delimited source; never infer a table.
    let trimmed = source.trim();
    if !trimmed.contains('|') {
        return None;
    }
    let cells: Vec<String> = trimmed
        .split('|')
        .map(str::trim)
        .filter(|cell| !cell.is_empty())
        .map(str::to_owned)
        .collect();
    (!cells.is_empty()).then_some(cells)
}

fn reservation_subject(
    source: &str,
    cells: Option<&[String]>,
) -> Option<(&'static str, &'static str, String)> {
    if let Some(cells) = cells {
        // A status may occupy its own cell, or a flattened row may
        // contain a complete declaration in one cell.
        let nonempty: Vec<&str> = cells
            .iter()
            .map(String::as_str)
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect();
        let statuses: Vec<usize> = nonempty
            .iter()
            .enumerate()
            .filter_map(|(i, cell)| {
                let status = cell.to_ascii_lowercase();
                (status == "reserved" || status == "reserved for future use").then_some(i)
            })
            .collect();
        if statuses.len() == 1 && nonempty.len() == 2 {
            let subject = explicit_subject(nonempty[1 - statuses[0]]);
            let declaration = format!("{subject} Reserved");
            if let Some((kind, scope)) = reserved_declaration(&declaration) {
                return Some((kind, scope, subject.to_owned()));
            }
        }
        if statuses.is_empty() && nonempty.len() == 1 {
            let cell = nonempty[0];
            if let Some((kind, scope)) = reserved_declaration(cell) {
                let index = cell.to_ascii_lowercase().find("reserved")?;
                let subject = explicit_subject(cell[..index].trim());
                if !subject.is_empty() {
                    return Some((kind, scope, subject.to_owned()));
                }
            }
        }
        return None;
    }
    let (kind, scope) = reserved_declaration(source)?;
    let lower = source.to_ascii_lowercase();
    let prefix = lower.find("reserved").map(|index| source[..index].trim())?;
    Some((kind, scope, explicit_subject(prefix).to_owned()))
}

fn normalized_subject(subject: &str, kind: &str) -> Value {
    let value = subject.trim().trim_start_matches('#').trim();
    let lower = value.to_ascii_lowercase();
    let range = [" - ", " – ", " to ", "..", "-", "–"]
        .iter()
        .find_map(|separator| lower.find(separator).map(|index| (index, *separator)));
    if let Some((index, separator)) = range {
        let first = value[..index].trim();
        let last = value[index + separator.len()..].trim();
        if !first.is_empty() && !last.is_empty() {
            return json!({"notation": "range", "start": first, "end": last,
                "expanded": false, "case_preserved": true});
        }
    }
    json!({"notation": if value.is_empty() { "unspecified" } else if kind == "bit_field" {
        "bit_field"
    } else { "single" }, "value": value, "expanded": false, "case_preserved": true})
}

pub(super) fn classify(document: &str) -> Value {
    let tree = SourceTree::parse(document);
    let lines: Vec<&str> = document.lines().collect();
    let mut regions = Vec::new();
    for section in &tree.sections {
        let Some(category) = lines.get(section.heading_line).and_then(|s| kind(s)) else {
            continue;
        };
        let end = ((section.heading_line + 1)..section.end_line.min(lines.len()))
            .find(|&line| {
                lines[line].trim_start().starts_with('#')
                    && !matches!(
                        tree.block_at_line(line).map(|b| b.kind),
                        Some(BlockKind::FencedCode)
                    )
            })
            .unwrap_or(section.end_line.min(lines.len()));
        let mut entries = 0usize;
        let mut references = 0usize;
        for (line, text) in lines
            .iter()
            .enumerate()
            .take(end)
            .skip(section.heading_line + 1)
        {
            if matches!(
                tree.block_at_line(line).map(|b| b.kind),
                Some(BlockKind::FencedCode)
            ) {
                continue;
            }
            if !text.trim().is_empty() {
                entries += 1;
                if reference_like(text) {
                    references += 1;
                }
            }
        }
        let confirmed = entries >= 3 && references >= 3 && references * 2 >= entries;
        regions.push(json!({
            "kind": category,
            "heading_line": section.heading_line + 1,
            "start_line": section.heading_line + 1,
            "end_line_exclusive": end + 1,
            "entries": entries,
            "reference_like_entries": references,
            "classification": if confirmed { "structurally_supported" } else { "unconfirmed_heading" },
            "exclude_from_extraction": false
        }));
    }
    let mut reservations = Vec::new();
    for (line, source) in lines.iter().enumerate() {
        if matches!(
            tree.block_at_line(line).map(|block| block.kind),
            Some(BlockKind::FencedCode)
        ) {
            continue;
        }
        // Navigation entries are references, not definitions.
        if regions.iter().any(|region| {
            region["classification"] == "structurally_supported"
                && region["start_line"]
                    .as_u64()
                    .is_some_and(|start| start <= (line + 1) as u64)
                && region["end_line_exclusive"]
                    .as_u64()
                    .is_some_and(|end| (line + 1) < end as usize)
        }) {
            continue;
        }
        let row_cells = tree.block_at_line(line).and_then(|block| {
            if let BlockKind::Table(index) = block.kind {
                tree.tables[index]
                    .rows
                    .iter()
                    .find(|row| row.line == line)
                    .map(|row| row.cells.as_slice())
            } else {
                None
            }
        });
        let fallback_cells = if row_cells.is_none() {
            pipe_cells(source)
        } else {
            None
        };
        let cells = row_cells.or(fallback_cells.as_deref());
        if let Some((subject_kind, scope, subject)) = reservation_subject(source, cells) {
            let context = tree.context_at_line(line);
            let section_path: Vec<Value> = context.as_ref().map_or_else(Vec::new, |ctx| {
                ctx.section_path
                    .iter()
                    .map(|&index| {
                        let section = &tree.sections[index];
                        json!({
                            "heading_line": section.heading_line + 1,
                            "heading": lines[section.heading_line].trim(),
                            "end_line_exclusive": section.end_line + 1
                        })
                    })
                    .collect()
            });
            let block = tree.block_at_line(line);
            let (container_kind, container_start, container_end, row_line) =
                match block.map(|block| block.kind) {
                    Some(BlockKind::Table(index)) => {
                        let table = &tree.tables[index];
                        (
                            "table_row",
                            table.start_line + 1,
                            table.end_line + 1,
                            Some(line + 1),
                        )
                    }
                    Some(BlockKind::Prose) => ("source_line", line + 1, line + 2, None),
                    _ => ("heading_or_source_line", line + 1, line + 2, None),
                };
            // A reservation only governs the declaration line (or the matching
            // table row). Section ancestry is context, never an ownership span.
            let declaration_span = json!({
                "start_line": line + 1,
                "end_line_exclusive": line + 2,
                "container_kind": container_kind,
                "container_start_line": container_start,
                "container_end_line_exclusive": container_end,
                "table_row_line": row_line,
                "section_path": section_path
            });
            reservations.push(json!({
                "kind": "reserved_declaration",
                "line": line + 1,
                "evidence": source.trim(),
                "subject_kind": subject_kind,
                "scope": scope,
                "subject": normalized_subject(&subject, subject_kind),
                "structural_scope": declaration_span,
                "classification": "lexical_candidate",
                "exclude_from_extraction": false
            }));
        }
    }
    json!({"version": 1, "read_only": true, "regions": regions, "reserved_declarations": reservations})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_generic_reservation_declarations() {
        let source = "# Definitions\nj - p Reserved\nBits 7:4 reserved\n0x80-0xFF reserved for future use\nReserved for future use\n";
        let report = classify(source);
        assert_eq!(report["reserved_declarations"].as_array().unwrap().len(), 4);
        assert_eq!(
            report["reserved_declarations"][0]["subject_kind"],
            "identifier_or_range"
        );
        assert_eq!(
            report["reserved_declarations"][1]["subject_kind"],
            "bit_field"
        );
        assert_eq!(
            report["reserved_declarations"][2]["subject_kind"],
            "value_range"
        );
        assert_eq!(report["reserved_declarations"][3]["scope"], "unscoped");
    }

    #[test]
    fn avoids_historical_descriptive_and_navigation_reservations() {
        let source = "# Contents\nReserved.... 1\nOther.... 2\nMore.... 3\n# Body\nReserved bandwidth\nPreviously reserved, now assigned\nNot reserved\n```text\nBits 7:4 reserved\n```\n";
        let report = classify(source);
        assert_eq!(report["reserved_declarations"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn reservation_scope_does_not_leak_into_following_sections() {
        let source = "# Messages\n## Reserved\nj - p Reserved\n## Active\nType: G\n";
        let report = classify(source);
        let item = report["reserved_declarations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["line"] == 3)
            .expect("explicit range declaration on line 3");
        assert_eq!(item["structural_scope"]["start_line"], 3);
        assert_eq!(item["structural_scope"]["end_line_exclusive"], 4);
        assert_eq!(
            item["structural_scope"]["section_path"][1]["heading"],
            "## Reserved"
        );
        assert_eq!(item["exclude_from_extraction"], false);
    }

    #[test]
    fn table_reservation_is_limited_to_matching_row() {
        let source =
            "# Fields\n| Code | Status |\n| --- | --- |\n| j - p | Reserved |\n| G | Active |\n";
        let report = classify(source);
        assert_eq!(report["reserved_declarations"].as_array().unwrap().len(), 1);
        let declaration = &report["reserved_declarations"][0];
        assert_eq!(
            declaration["structural_scope"]["container_kind"],
            "table_row"
        );
        assert_eq!(declaration["structural_scope"]["start_line"], 4);
        assert_eq!(declaration["structural_scope"]["end_line_exclusive"], 5);
        assert_eq!(declaration["subject"]["start"], "j");
        assert_eq!(declaration["subject"]["end"], "p");
    }

    #[test]
    fn reversed_table_columns_and_case_are_preserved() {
        let source =
            "# Codes\n| Status | Code |\n| --- | --- |\n| Reserved | J - P |\n| Active | G |\n";
        let report = classify(source);
        assert_eq!(report["reserved_declarations"].as_array().unwrap().len(), 1);
        assert_eq!(report["reserved_declarations"][0]["subject"]["start"], "J");
        assert_eq!(report["reserved_declarations"][0]["subject"]["end"], "P");
    }

    #[test]
    fn ambiguous_multi_subject_rows_are_not_claimed() {
        let source = "# Codes\n| Code | Status | Notes |\n| --- | --- | --- |\n| J | Reserved | X |\n| G | Active | Reserved |\n";
        let report = classify(source);
        assert_eq!(report["reserved_declarations"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn numeric_and_bit_ranges_are_not_expanded() {
        let report = classify("# Ranges\n0x80-0xFF Reserved\nBits 7:4 reserved\n");
        assert_eq!(
            report["reserved_declarations"][0]["subject"]["start"],
            "0x80"
        );
        assert_eq!(report["reserved_declarations"][0]["subject"]["end"], "0xFF");
        assert_eq!(
            report["reserved_declarations"][0]["subject"]["expanded"],
            false
        );
        assert_eq!(
            report["reserved_declarations"][1]["subject"]["notation"],
            "bit_field"
        );
    }

    #[test]
    fn flattened_type_status_rows_extract_exact_identifier() {
        let source = "# Messages\nType: D | Reserved |  |\nType: b | Reserved |  |\n";
        let report = classify(source);
        assert_eq!(report["reserved_declarations"].as_array().unwrap().len(), 2);
        assert_eq!(report["reserved_declarations"][0]["subject"]["value"], "D");
        assert_eq!(report["reserved_declarations"][1]["subject"]["value"], "b");
    }

    #[test]
    fn flattened_single_cell_reservations_survive_empty_cells() {
        let source = "# Messages\n| t Reserved |  |\n| u Reserved |  |\n| v Active |  |\n";
        let report = classify(source);
        assert_eq!(report["reserved_declarations"].as_array().unwrap().len(), 2);
        assert_eq!(report["reserved_declarations"][0]["subject"]["value"], "t");
        assert_eq!(report["reserved_declarations"][1]["subject"]["value"], "u");
    }

    #[test]
    fn ambiguous_pipe_row_does_not_claim_reservation() {
        let report = classify("# Messages\n| D | Reserved | Other |\n");
        assert_eq!(report["reserved_declarations"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn toc_requires_multiple_references() {
        let source =
            "# Contents\nOverview .... 1\nDetails .... 2\nAppendix .... 9\n# Body\nType: G\n";
        let report = classify(source);
        assert_eq!(
            report["regions"][0]["classification"],
            "structurally_supported"
        );
        assert_eq!(report["regions"][0]["exclude_from_extraction"], false);
    }

    #[test]
    fn toc_supports_roman_pages_and_attached_dot_leaders() {
        let source = "# Contents\n- About. vii\n- Audience. viii\n- Chapter 3 Compatibility....43\n- 3.1 Version Compatibility....43\n# Body\n";
        let report = classify(source);
        assert_eq!(report["regions"][0]["reference_like_entries"], 4);
        assert_eq!(
            report["regions"][0]["classification"],
            "structurally_supported"
        );
    }

    #[test]
    fn index_heading_without_entries_is_not_confirmed() {
        let report = classify("# Index\nThis is a database index.\n");
        assert_eq!(
            report["regions"][0]["classification"],
            "unconfirmed_heading"
        );
    }

    #[test]
    fn fenced_heading_is_not_a_navigation_region() {
        let report = classify("~~~markdown\n# Contents\nEntry 1\nEntry 2\nEntry 3\n~~~\n");
        assert_eq!(report["regions"].as_array().unwrap().len(), 0);
    }
}
