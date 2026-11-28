use super::audit_parameter_row_kind;

fn kind(first: &str, second: &str) -> &'static str {
    audit_parameter_row_kind(&[first.to_owned(), second.to_owned()])
}

#[test]
fn reserved_markers_do_not_require_single_letter_identifiers() {
    assert_eq!(kind("t Reserved", ""), "reserved_marker_candidate");
    assert_eq!(kind("u Reserved", ""), "reserved_marker_candidate");
    assert_eq!(kind("T Reserved", ""), "reserved_marker_candidate");
    assert_eq!(kind("q Reserved", ""), "reserved_marker_candidate");
    assert_eq!(kind("Z reserved", ""), "reserved_marker_candidate");
    assert_eq!(kind("aa Reserved", ""), "reserved_marker_candidate");
    assert_eq!(kind("q Available", ""), "unclassified_row");
}

#[test]
fn continuation_and_prose_do_not_acquire_parameter_ownership() {
    assert_eq!(
        kind("(Optional key value pairs)", ""),
        "possible_continuation"
    );
    assert_eq!(kind("Frequency is in MHz.", "string"), "unclassified_row");
    assert_eq!(
        kind("Reference required to be present on Rx", ""),
        "unclassified_row"
    );
}

#[test]
fn row_classification_reports_lexical_patterns() {
    assert_eq!(
        kind("Time to Blockage (blockTime)", "blockTime=&lt;int sec>"),
        "named_key_candidate"
    );
    assert_eq!(
        kind(
            "Additional Antenna Status (statusCode)",
            "statusCode=&lt;int 0 to 65535>"
        ),
        "named_key_candidate"
    );
    assert_eq!(
        kind("Time to Blockage (blockTime)", "other=1"),
        "unclassified_row"
    );
    assert_eq!(kind("blockTime", "int seconds"), "single_token_first_cell");
    assert_eq!(kind("statusCode", "int"), "single_token_first_cell");
    assert_eq!(kind("blockTime", ""), "unclassified_row");
}

#[test]
fn numeric_codes_and_names_have_no_inferred_entity_type() {
    assert_eq!(kind("17", "Failure"), "single_token_first_cell");
    assert_eq!(kind("0x03", "Read"), "single_token_first_cell");
    assert_eq!(kind("CONNECT", "Reserved for future use"), "single_token_first_cell");
    assert_eq!(kind("0x03 Reserved", ""), "reserved_marker_candidate");
}
