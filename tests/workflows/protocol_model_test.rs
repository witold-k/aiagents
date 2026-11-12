use super::*;

use super::super::message_identity::CandidateFact;

fn candidate(quote: &str, offset: Option<usize>, ambiguous: bool) -> MessageCandidate {
    MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "unverified".into(),
        facts: vec![CandidateFact {
            statement: "candidate fact".into(),
            segment_id: "segment-0".into(),
            evidence_quote: Some(quote.into()),
            source_byte_offset: offset,
            source_locator_ambiguous: ambiguous,
            statement_byte_offset: None,
        }],
    }
}

#[test]
fn exact_assignment_is_only_a_lexical_hint() {
    let report = stage(
        &[candidate("replyTo=getExtAntStatus", Some(0), false)],
        &["replyTo=getExtAntStatus"],
        &simplelexer::tablelexer::DocumentAnalysis::parse("replyTo=getExtAntStatus"),
    );
    let message = &report.messages[0];
    assert_eq!(message.assertions[0].source_quote_status, "exact");
    assert_eq!(message.parameter_hints[0].name, "replyTo");
    assert_eq!(message.parameter_hints[0].status, "lexical_hypothesis");
    assert_eq!(message.parameter_hints[0].value_type, None);
}

#[test]
fn missing_or_ambiguous_quote_cannot_produce_parameter_hint() {
    for (offset, ambiguous) in [(None, false), (Some(0), true), (Some(1), false)] {
        let report = stage(
            &[candidate("statusCode=18", offset, ambiguous)],
            &["statusCode=18"],
            &simplelexer::tablelexer::DocumentAnalysis::parse("statusCode=18"),
        );
        assert!(report.messages[0].parameter_hints.is_empty());
        assert_eq!(
            report.messages[0].assertions[0].source_quote_status,
            "unverified"
        );
    }
}

#[test]
fn prose_and_table_caption_are_not_parameter_hints() {
    let report = stage(
        &[candidate("Table A-5. Transceiver", Some(0), false)],
        &["Table A-5. Transceiver"],
        &simplelexer::tablelexer::DocumentAnalysis::parse("Table A-5. Transceiver"),
    );
    assert!(report.messages[0].parameter_hints.is_empty());
}

#[test]
fn explicit_type_syntax_is_distinct_from_example_value() {
    for (quote, expected_kind, expected_type) in [
        ("statusCode=<int 0 to 65535>", "definition_syntax", Some("int")),
        ("statusCode=&lt;int 0 to 65535>", "definition_syntax", Some("int")),
        ("statusCode=20", "assignment_example_or_value", None),
    ] {
        let report = stage(&[candidate(quote, Some(0), false)], &[quote], &simplelexer::tablelexer::DocumentAnalysis::parse(quote));
        let hint = &report.messages[0].parameter_hints[0];
        assert_eq!(hint.name, "statusCode");
        assert_eq!(hint.syntax_kind, expected_kind);
        assert_eq!(hint.value_type.as_deref(), expected_type);
        assert!(hint.constraints.is_empty());
        assert_eq!(hint.required, None);
    }
}

#[test]
fn a_type_looking_string_without_an_exact_anchor_is_not_promoted() {
    let report = stage(
        &[candidate("statusCode=<int 0 to 65535>", Some(1), false)],
        &["statusCode=<int 0 to 65535>"],
        &simplelexer::tablelexer::DocumentAnalysis::parse("statusCode=<int 0 to 65535>"),
    );
    assert!(report.messages[0].parameter_hints.is_empty());
}

#[test]
fn normalized_table_evidence_is_observed_without_claiming_ownership() {
    let source = "| Parameter | Meaning |\n| --- | --- |\n| statusCode | status |\n";
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(source);
    let report = stage(&[], &[], &analysis);
    assert!(report.messages.is_empty());
    assert_eq!(report.table_evidence.len(), 1);
    assert!(report.table_evidence[0].rows.iter().any(|row|
        row.cells.iter().any(|cell| cell == "statusCode")
    ));
}
