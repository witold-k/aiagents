use super::*;

use super::super::message_identity::CandidateFact;

#[test]
fn exact_quote_is_not_treated_as_validated_schema() {
    let source = "statusCode=18";
    let candidate = MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "status".into(),
        facts: vec![CandidateFact {
            statement: "status code".into(),
            segment_id: "segment".into(),
            evidence_quote: Some(source.into()),
            source_byte_offset: Some(0),
            source_locator_ambiguous: false,
            statement_byte_offset: None,
        }],
    };
    let report = audit(&[candidate], &[source]);
    assert_eq!(report.exact_source_anchored_facts, 1);
    assert_eq!(report.unanchored_or_ambiguous_facts, 0);
    assert_eq!(
        report.candidate_inventory[0].field_schema_status,
        "not_derived"
    );
}

#[test]
fn ambiguous_anchor_is_not_counted() {
    let candidate = MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "status".into(),
        facts: vec![CandidateFact {
            statement: "status code".into(),
            segment_id: "segment".into(),
            evidence_quote: Some("statusCode=18".into()),
            source_byte_offset: Some(0),
            source_locator_ambiguous: true,
            statement_byte_offset: None,
        }],
    };
    let report = audit(&[candidate], &["statusCode=18"]);
    assert_eq!(report.exact_source_anchored_facts, 0);
    assert_eq!(report.unanchored_or_ambiguous_facts, 1);
}
