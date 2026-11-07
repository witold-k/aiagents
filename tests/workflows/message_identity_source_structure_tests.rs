use super::SourceStructure;

#[test]
fn table_tree_owns_flattened_rows_and_respects_next_declaration() {
    let source = "## Message Types\nType: v | status | value\ncontinuation\nmore details\nType: V | request | key\nmore details\n## Semantics\nnot a row";
    let tree = SourceStructure::from_document(source);
    assert_eq!(tree.table_type(3), Some(b'v'));
    assert_eq!(tree.table_type(5), Some(b'V'));
    assert_eq!(tree.table_type(7), None);
}

#[test]
fn table_tree_does_not_inherit_malformed_declarations_or_pipe_prose() {
    let source = "## Message Types\nType: v | status\nType: ?? | unknown\ncontinuation\n## Other\n| ordinary | prose |";
    let tree = SourceStructure::from_document(source);
    assert_eq!(tree.table_type(1), Some(b'v'));
    assert_eq!(tree.table_type(3), None);
    assert_eq!(tree.table_type(5), None);
}

#[test]
fn owns_immediate_continuation_of_fenced_command() {
    let source = "## Messages from Modem to Antenna\n```txt\nB 100 200 rx2=300\ntx3=400 # additional frequency\n```";
    let structure = SourceStructure::from_document(source);
    assert_eq!(structure.example_type(2), Some(b'B'));
    assert_eq!(structure.example_type(3), Some(b'B'));
}

#[test]
fn never_inherits_across_blank_lines_fences_or_sections() {
    let source = "## Messages from Modem to Antenna\n```txt\nB 100 # example\n\ntx3=400 # detached\n```\n## Appendix\n```txt\nB 100 # unrelated\n```";
    let structure = SourceStructure::from_document(source);
    assert_eq!(structure.example_type(4), None);
    assert_eq!(structure.example_type(8), None);
}

#[test]
fn preserves_case_and_does_not_claim_arbitrary_prose() {
    let source = "## Messages from Modem to Antenna\n```txt\nb 100 # command\nplain prose # comment\nQ 20 # different command\n```";
    let structure = SourceStructure::from_document(source);
    assert_eq!(structure.example_type(2), Some(b'b'));
    assert_eq!(structure.example_type(3), None);
    assert_eq!(structure.example_type(4), Some(b'Q'));
}
