use super::explicit_message_mentions;

#[test]
fn recognizes_multiple_explicit_quoted_message_mentions() {
    assert_eq!(
        explicit_message_mentions(
            r#"When the controller receives an "F" message, it must respond with an "s" message."#
        ),
        vec!['F', 's']
    );
}

#[test]
fn does_not_infer_identity_from_incidental_letters_or_unquoted_words() {
    assert!(
        explicit_message_mentions(
            r#"The F message changes state. 'X' setting, 'F' field and "s" status."#
        )
        .is_empty()
    );
}

#[test]
fn recognizes_enumerated_commands_with_unicode_quotes() {
    assert_eq!(
        explicit_message_mentions("If the modem issues a “P”, ‘B’, or \"F\" command"),
        vec!['B', 'F', 'P']
    );
}

#[test]
fn does_not_include_unrelated_quoted_tokens_in_a_list() {
    assert_eq!(
        explicit_message_mentions("'X' setting and 'P', 'B', or 'F' command"),
        vec!['B', 'F', 'P']
    );
    assert!(explicit_message_mentions("'X', 'Y' settings").is_empty());
}

#[test]
fn identifies_protocol_examples_separately() {
    assert_eq!(
        super::protocol_example_prefix("  W 1 # periodic report"),
        Some('W')
    );
    assert_eq!(super::protocol_example_prefix("The W command # note"), None);
    assert_eq!(super::protocol_example_prefix("W 1"), None);
}

#[test]
fn recognizes_quoted_command_without_duplicate_mentions() {
    assert_eq!(
        explicit_message_mentions("The 'K' command and 'K' message."),
        vec!['K']
    );
}
