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
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(source);
    assert_eq!(html_table_spans(source, &analysis), vec![(1, 4)]);
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
