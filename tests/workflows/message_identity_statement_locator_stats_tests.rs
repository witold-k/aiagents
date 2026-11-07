use super::*;

#[test]
fn counts_exact_missing_and_bad_positions_without_identity_changes() {
    let candidate = MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "messages/unknown.md".into(),
        facts: [("beta", Some(8)), ("alpha", None), ("beta", Some(2))]
            .into_iter()
            .map(|(statement, statement_byte_offset)| CandidateFact {
                statement: statement.into(),
                segment_id: "S1".into(),
                evidence_quote: Some("alpha".into()),
                source_byte_offset: Some(0),
                source_locator_ambiguous: false,
                statement_byte_offset,
            })
            .collect(),
    };
    let stats = statement_locator_stats(
        &[candidate],
        &["alpha | beta"],
        &[10],
        "0123456789alpha | beta",
    );
    assert_eq!(stats.total, 3);
    assert_eq!(stats.located, 2);
    assert_eq!(stats.missing, 1);
    assert_eq!(stats.exact_match, 1);
    assert_eq!(stats.mismatch, 1);
    assert_eq!(stats.different_segment, 2);
}
