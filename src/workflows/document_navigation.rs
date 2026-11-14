// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Read-only navigation region detection. Headings alone never suppress facts.
use super::source_tree::{BlockKind, SourceTree};
use serde_json::{json, Value};

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
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') { return false; }
    let last = s.split_whitespace().last().unwrap_or("");
    let page = !last.is_empty() && last.chars().all(|c| c.is_ascii_digit());
    let comma_pages = s.rsplit_once(',').is_some_and(|(_, tail)| {
        let tail = tail.trim();
        !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit() || c == '-' || c == ' ')
    });
    (page && s.len() > last.len() + 2) || s.contains("](#") || s.contains("](http") || comma_pages
}

pub(super) fn classify(document: &str) -> Value {
    let tree = SourceTree::parse(document);
    let lines: Vec<&str> = document.lines().collect();
    let mut regions = Vec::new();
    for section in &tree.sections {
        let Some(category) = lines.get(section.heading_line).and_then(|s| kind(s)) else { continue; };
        let end = ((section.heading_line + 1)..section.end_line.min(lines.len()))
            .find(|&line| lines[line].trim_start().starts_with('#')
                && !matches!(tree.block_at_line(line).map(|b| b.kind), Some(BlockKind::FencedCode)))
            .unwrap_or(section.end_line.min(lines.len()));
        let mut entries = 0usize;
        let mut references = 0usize;
        for line in (section.heading_line + 1)..end {
            if matches!(tree.block_at_line(line).map(|b| b.kind), Some(BlockKind::FencedCode)) { continue; }
            if !lines[line].trim().is_empty() {
                entries += 1;
                if reference_like(lines[line]) { references += 1; }
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
    json!({"version": 1, "read_only": true, "regions": regions})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toc_requires_multiple_references() {
        let source = "# Contents\nOverview .... 1\nDetails .... 2\nAppendix .... 9\n# Body\nType: G\n";
        let report = classify(source);
        assert_eq!(report["regions"][0]["classification"], "structurally_supported");
        assert_eq!(report["regions"][0]["exclude_from_extraction"], false);
    }

    #[test]
    fn index_heading_without_entries_is_not_confirmed() {
        let report = classify("# Index\nThis is a database index.\n");
        assert_eq!(report["regions"][0]["classification"], "unconfirmed_heading");
    }

    #[test]
    fn fenced_heading_is_not_a_navigation_region() {
        let report = classify("~~~markdown\n# Contents\nEntry 1\nEntry 2\nEntry 3\n~~~\n");
        assert_eq!(report["regions"].as_array().unwrap().len(), 0);
    }
}
