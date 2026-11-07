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
    );
    assert!(report.messages[0].parameter_hints.is_empty());
}
