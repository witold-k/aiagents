use super::*;

use crate::workflows::chunked_transform::FactReview;

fn fact(statement: &str, quote: &str) -> SourceFact {
    SourceFact {
        statement: statement.into(),
        source_chunk: 0,
        start_line: 1,
        end_line: 1,
        evidence_quote: Some(quote.into()),
        source_byte_offset: None,
        evidence_occurrences: vec![],
        review: FactReview::Supported,
    }
}

#[test]
fn condition_loss_is_warning_only() {
    let mut facts = BTreeMap::new();
    facts.insert(
        "messages/x.md".into(),
        vec![
            fact("Stop the motor.", "If the sensor fails, stop the motor."),
            fact(
                "If the sensor fails, stop the motor.",
                "If the sensor fails, stop the motor.",
            ),
        ],
    );
    let warnings = diagnose(&facts);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].kind, "possible_lost_condition");
    assert_eq!(warnings[0].fact_index, 0);
    assert_eq!(facts["messages/x.md"][0].review, FactReview::Supported);
}

#[test]
fn detects_joined_generic_declarations() {
    let mut facts = BTreeMap::new();
    facts.insert(
        "messages/y.md".into(),
        vec![fact("float pressure, kPafloat temperature, C", "")],
    );
    assert_eq!(diagnose(&facts)[0].kind, "possible_joined_declarations");
}

#[test]
fn no_evidence_does_not_invent_conditions() {
    let mut facts = BTreeMap::new();
    facts.insert("general.md".into(), vec![fact("The motor stops.", "")]);
    assert!(diagnose(&facts).is_empty());
}

#[test]
fn unrelated_condition_is_a_warning_not_a_rejection() {
    let mut facts = BTreeMap::new();
    facts.insert(
        "items/sensor.md".into(),
        vec![fact(
            "The device has a reset button.",
            "The device has a reset button. If the sensor fails, stop it.",
        )],
    );
    assert_eq!(diagnose(&facts)[0].kind, "possible_lost_condition");
    assert_eq!(facts["items/sensor.md"][0].review, FactReview::Supported);
}

#[test]
fn unresolved_fact_is_reported_with_arbitrary_path() {
    let mut facts = BTreeMap::new();
    let mut entry = fact("X", "");
    entry.review = FactReview::Unresolved;
    facts.insert("items/sensor.md".into(), vec![entry]);
    let findings = diagnose(&facts);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].path, "items/sensor.md");
    assert_eq!(findings[0].fact_index, 0);
    assert_eq!(findings[0].kind, "unresolved");
}

#[test]
fn reviewed_reference_fixture_has_contiguous_typed_parameters() {
    // Benchmark data, not runtime knowledge or protocol-specific logic.
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/extraction_quality/synthetic_protocol_reference.json"
    ))
    .expect("valid reference fixture");
    let messages = fixture["messages"].as_array().expect("messages array");
    assert!(!messages.is_empty());
    for message in messages {
        let parameters = message["parameters"].as_array().expect("parameters array");
        for (index, parameter) in parameters.iter().enumerate() {
            assert_eq!(parameter["position"].as_u64(), Some(index as u64 + 1));
            assert!(
                parameter["name"]
                    .as_str()
                    .is_some_and(|name| !name.is_empty())
            );
            assert!(
                parameter["type"]
                    .as_str()
                    .is_some_and(|kind| !kind.is_empty())
            );
        }
    }
}
