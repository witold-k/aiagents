use super::message_example_type_for_line;

#[test]
fn recognizes_only_message_examples_and_preserves_case() {
    let source = "## Messages from Modem to Antenna\n```txt\nB 100 200 # frequency\na 60 # alive\n```\n## Other examples\n```txt\nZ 100\n```";
    assert_eq!(message_example_type_for_line(source, 2), Some(b'B'));
    assert_eq!(message_example_type_for_line(source, 3), Some(b'a'));
    assert_eq!(message_example_type_for_line(source, 7), None);
}

#[test]
fn rejects_prose_continuations_and_unfenced_commands() {
    let source =
        "## Messages from Controller\nA 10\n```txt\nA 10 # ping\ncontinued line\n# comment\n```";
    assert_eq!(message_example_type_for_line(source, 1), None);
    assert_eq!(message_example_type_for_line(source, 3), Some(b'A'));
    assert_eq!(message_example_type_for_line(source, 4), None);
    assert_eq!(message_example_type_for_line(source, 5), None);
}
