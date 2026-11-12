use super::*;

#[test]
fn parses_generic_nested_sections_and_tables() {
    let doc = "# Inventory\n## Components\n| Part | Count |\n| :--- | ---: |\n| bolt | 42 |\n### Notes\nText\n## Other\nEnd";
    let tree = SourceTree::parse(doc);
    assert_eq!(tree.sections.len(), 4);
    assert_eq!(tree.sections[1].parent, Some(0));
    assert_eq!(tree.sections[2].parent, Some(1));
    assert_eq!(tree.sections[1].end_line, 7);
    assert_eq!(tree.tables.len(), 1);
    assert_eq!(tree.tables[0].rows[1].cells, ["bolt", "42"]);
    assert_eq!(
        tree.block_at_line(4).map(|b| b.kind),
        Some(BlockKind::Table(0))
    );
    assert_eq!(
        tree.block_at_line(6).map(|b| b.kind),
        Some(BlockKind::Prose)
    );
}

#[test]
fn pipes_in_prose_and_code_are_not_tables() {
    let doc = "# Notes\nA | B\nSome | text\n\n~~~rust\n| not | a table |\n~~~\nAfter";
    let tree = SourceTree::parse(doc);
    assert!(tree.tables.is_empty());
    assert_eq!(
        tree.block_at_line(5).map(|b| b.kind),
        Some(BlockKind::FencedCode)
    );
    assert_eq!(
        tree.block_at_line(7).map(|b| b.kind),
        Some(BlockKind::Prose)
    );
}

#[test]
fn table_stops_at_prose_and_restarts_only_with_separator() {
    let doc = "| Item | Price |\n| --- | --- |\n| book | 9 |\nThis is commentary\n| another | row |\n| but | no header |";
    let tree = SourceTree::parse(doc);
    assert_eq!(tree.tables.len(), 1);
    assert_eq!(tree.tables[0].end_line, 3);
    assert_eq!(
        tree.block_at_line(4).map(|b| b.kind),
        Some(BlockKind::Prose)
    );
}

#[test]
fn headings_inside_fences_do_not_create_sections() {
    let doc = "# One\n```md\n## Not a section\n```\n## Two";
    let tree = SourceTree::parse(doc);
    assert_eq!(tree.sections.len(), 2);
    assert_eq!(tree.sections[1].parent, Some(0));
}
#[test]
fn chunk_boundaries_do_not_change_the_tree() {
    let doc = "# Catalog\n## Parts\n| Part | Qty |\n| --- | --- |\n| nut | 4 |\n| bolt | 9 |\n\n~~~md\n## Not a section\n~~~\n## Appendix\nRésumé 🧪";
    let expected = SourceTree::parse(doc);
    for split in 0..=doc.len() {
        let mut parser = SourceParser::new();
        parser.push_bytes(&doc.as_bytes()[..split]);
        parser.push_bytes(&doc.as_bytes()[split..]);
        assert_eq!(parser.finish(), expected, "split at byte {split}");
    }
    let mut parser = SourceParser::new();
    for byte in doc.as_bytes() {
        parser.push_bytes(std::slice::from_ref(byte));
    }
    assert_eq!(parser.finish(), expected);
}

#[test]
fn source_context_is_independent_of_chunk_layout() {
    let doc = "# Catalog\n## Parts\n| Part | Qty |\n| --- | --- |\n| nut | 4 |\n## Other\nText";
    let tree = SourceTree::parse(doc);
    let context = tree.context_at_line(4).expect("table context");
    assert_eq!(context.section_path, vec![0, 1]);
    assert_eq!(context.block, Some(BlockKind::Table(0)));
    let context = tree.context_at_line(6).expect("prose context");
    assert_eq!(context.section_path, vec![0, 2]);
    assert_eq!(context.block, Some(BlockKind::Prose));
    assert!(tree.context_at_line(7).is_none());
}

#[test]
fn a_chunk_boundary_is_not_a_table_boundary() {
    let mut parser = SourceParser::new();
    parser.push_chunk("# Shop\n| Item | Price |\n");
    parser.push_chunk("| --- | --- |\n| book");
    parser.push_chunk(" | 9 |\n| pen | 2 |\n");
    let tree = parser.finish();
    assert_eq!(tree.tables.len(), 1);
    assert_eq!(tree.tables[0].rows.len(), 3);
    assert_eq!(tree.tables[0].rows[0].line, 1);
    assert_eq!(tree.tables[0].rows[1].line, 3);
    assert_eq!(tree.tables[0].rows[2].line, 4);
    assert_eq!(tree.tables[0].end_line, 5);
}
#[test]
fn transitions_are_source_positioned_and_chunk_independent() {
    let doc = "# Parts\n| Part | Qty |\n| --- | --- |\n| nut | 2 |\nEnd";
    let expected = SourceTree::parse(doc).transitions();
    assert_eq!(
        expected[0],
        Transition {
            line: 0,
            kind: TransitionKind::SectionStart
        }
    );
    assert!(expected.contains(&Transition {
        line: 1,
        kind: TransitionKind::TableStart
    }));
    assert!(expected.contains(&Transition {
        line: 4,
        kind: TransitionKind::TableEnd
    }));
    let mut parser = SourceParser::new();
    for byte in doc.as_bytes() {
        parser.push_bytes(std::slice::from_ref(byte));
    }
    assert_eq!(parser.finish().transitions(), expected);
}
#[test]
fn layout_hypotheses_preserve_evidence_without_claiming_identity() {
    let doc = "# Inventory\nType: v | Description\ncontinued | text\n## Other\nSingle | pipe\n\n```text\na | b\nc | d\n```\n| Key | Value |\n| --- | --- |\n| x | y |";
    let tree = SourceTree::parse(doc);
    let hypotheses = tree.layout_hypotheses(doc);
    assert_eq!(hypotheses.len(), 1);
    let hypothesis = &hypotheses[0];
    assert_eq!((hypothesis.start_line, hypothesis.end_line), (1, 3));
    assert_eq!(hypothesis.status, HypothesisStatus::Proposed);
    assert_eq!(hypothesis.relations.len(), 1);
    assert_eq!(hypothesis.relations[0].kind, RelationKind::Continues);
    assert_eq!(hypothesis.relations[0].evidence_lines, vec![1, 2]);
    assert_eq!(hypothesis.relations[0].evidence, RelationEvidence::Syntax);
}

#[test]
fn layout_hypotheses_do_not_depend_on_parser_chunk_boundaries() {
    let doc = "# A\nfirst | row\nsecond | row\nthird | row";
    let expected = SourceTree::parse(doc).layout_hypotheses(doc);
    let mut parser = SourceParser::new();
    for byte in doc.as_bytes() {
        parser.push_bytes(std::slice::from_ref(byte));
    }
    assert_eq!(parser.finish().layout_hypotheses(doc), expected);
}

#[test]
fn deterministic_first_pass_builds_scoped_windows() {
    let source = "# Root\nintro\n## Alpha\nfirst\n## Beta\nsecond\n";
    let tree = SourceTree::parse(source);
    let scope = tree.document_scope(source);
    scope.validate(source).expect("valid byte spans");
    assert_eq!(scope.nodes.len(), 3);
    assert_eq!(scope.nodes[1].parent, Some(0));
    assert_eq!(scope.nodes[2].parent, Some(0));
    let budget = crate::document_understanding::WindowBudget {
        context_tokens: 128, output_tokens: 8,
        reserved_tokens: 8, prefix_tokens: 8, chars_per_token: 1,
    };
    let windows = scope.plan(source, budget).expect("plan");
    assert_eq!(windows.iter().map(|w| &source[w.start_byte..w.end_byte]).collect::<String>(), source);
    assert!(windows.iter().any(|w| w.prefix.contains("Alpha")));
    assert!(windows.iter().any(|w| w.prefix.contains("Beta")));
    assert!(windows.iter().filter(|w| w.prefix.contains("Beta")).all(|w| !w.prefix.contains("Alpha")));
}

#[test]
fn deterministic_scope_handles_unicode_and_no_headings() {
    let source = "äöü\nno headings\n";
    let scope = SourceTree::parse(source).document_scope(source);
    assert!(scope.nodes.is_empty());
    let windows = scope.plan(source, crate::document_understanding::WindowBudget {
        context_tokens: 12, output_tokens: 2,
        reserved_tokens: 2, prefix_tokens: 2, chars_per_token: 1,
    }).unwrap();
    assert_eq!(windows.iter().map(|w| &source[w.start_byte..w.end_byte]).collect::<String>(), source);
}
