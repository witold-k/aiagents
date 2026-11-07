use super::flattened_message_table_type;

#[test]
fn long_converted_table_row_keeps_its_explicit_identity() {
    let mut lines = vec![
        "## Message Types".to_owned(),
        "Type: q | status | 3 | int value".to_owned(),
    ];
    lines.extend((0..25).map(|n| format!("Continuation of parameter {n}")));
    let document = lines.join("\n");
    assert_eq!(flattened_message_table_type(&document, 26), Some(b'q'));
}

#[test]
fn new_row_and_heading_are_hard_boundaries() {
    let source = "## Message Types\nType: Q | description\ncontinuation\nType: q | response\ncontinuation\n## Appendix\nother prose";
    assert_eq!(flattened_message_table_type(source, 2), Some(b'Q'));
    assert_eq!(flattened_message_table_type(source, 4), Some(b'q'));
    assert_eq!(flattened_message_table_type(source, 6), None);
}

#[test]
fn damaged_declaration_does_not_inherit_previous_row() {
    let source = "## Message Types\nType: Q | description\nType: ?? | damaged\ncontinuation";
    assert_eq!(flattened_message_table_type(source, 3), None);
}
