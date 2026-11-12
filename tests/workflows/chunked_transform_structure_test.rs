use aiagents::workflows::chunked_transform::{declared_message_identity, evidence_segments};

#[test]
fn normalized_type_cells_establish_message_identity() {
    let source = "Protocol message definitions\n<table><tr><td>Type</td><td>Description</td><td>No. of Parameter(s)</td><td>Name of the Parameters</td></tr><tr><td>A</td><td>Heartbeat interval in seconds. Zero disables repeats.</td><td>1</td><td>int interval, seconds</td></tr></table>";
    let normalized = simplelexer::tablelexer::normalize_table_text(source).text;
    assert!(normalized.contains("Type: A | Heartbeat interval"));
    assert!(declared_message_identity("messages/a.md", &normalized));
    assert!(!declared_message_identity("messages/b.md", &normalized));
    assert!(!normalized.contains("<table"));
}

#[test]
fn example_blocks_are_not_identity_declarations() {
    let source =
        "Usage examples\n```txt\nA 10 # Alive\nB 18000.500 28000.500 rx2=18200\nL 1 1\n```\n";
    assert!(!declared_message_identity("messages/a.md", source));
    assert!(!declared_message_identity("messages/b.md", source));
    assert!(!declared_message_identity("messages/l.md", source));
}

#[test]
fn segment_ids_reference_normalized_table_row() {
    let source = simplelexer::tablelexer::normalize_table_text(
        "<table><tr><td>s</td><td>Status message</td></tr></table>",
    )
    .text;
    let segments = evidence_segments(&source);
    assert_eq!(segments[0].id, "S1");
    assert!(declared_message_identity(
        "messages/s.md",
        &segments[0].text
    ));
}
