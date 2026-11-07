use super::SourceTree;

#[test]
fn preserves_heading_hierarchy_and_independent_table_rows() {
    let source = "# Specification\n## Commands\n| Type | Description |\n| --- | --- |\n| v | status |\n### Parameters\n| Parameter | Format |\n| speed | float |\n## Semantics\nProse";
    let tree = SourceTree::from_document(source);
    assert_eq!(tree.section(4).map(|s| s.heading_line), Some(1));
    assert_eq!(tree.section(7).and_then(|s| s.parent), Some(1));
    assert_eq!(tree.section(9).map(|s| s.heading_line), Some(8));
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(source);
    assert!(
        analysis
            .source_row_at(source.find("| v |").unwrap())
            .is_some()
    );
    // This pipe-delimited fragment has no Markdown separator row.
    // Do not promote malformed table-looking text into a parsed table.
    assert!(
        analysis
            .source_row_at(source.find("| speed |").unwrap())
            .is_none()
    );
}

#[test]
fn flattened_table_continuation_inherits_only_explicit_row() {
    let source = "## Reserved\nType | Description | Parameters\nType: v | status | key\nContinuation one\nContinuation two\nType: V | request | key\nContinuation three\n## Semantics\nOrdinary prose";
    let tree = SourceTree::from_document(source);
    assert_eq!(tree.flattened_table_owner(source, 4), Some(b'v'));
    assert_eq!(tree.flattened_table_owner(source, 6), Some(b'V'));
    assert_eq!(tree.flattened_table_owner(source, 8), None);
}

#[test]
fn flattened_table_requires_header_and_valid_declaration() {
    let source = "## Notes\nType: v | text\nProse\nType | Description | Parameters\nType: ? | malformed\nMore prose\nType: v | valid\nContinuation";
    let tree = SourceTree::from_document(source);
    assert_eq!(tree.flattened_table_owner(source, 2), None);
    assert_eq!(tree.flattened_table_owner(source, 5), None);
    assert_eq!(tree.flattened_table_owner(source, 7), Some(b'v'));
}

#[test]
fn code_fences_and_prose_never_become_table_rows() {
    let source =
        "# Example\n```text\n| not | a table |\n```\nType: v | status\nprose\nType: V | request";
    let tree = SourceTree::from_document(source);
    assert_eq!(tree.section(2).map(|s| s.heading_line), Some(0));
    assert_eq!(tree.section(5).map(|s| s.heading_line), Some(0));
}
