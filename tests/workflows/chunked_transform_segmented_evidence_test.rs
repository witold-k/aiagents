use super::*;

#[test]
fn audit_review_is_not_reused_across_distinct_occurrences() {
    let mut previous = SourceFact {
        statement: "same assertion".into(),
        source_chunk: 0,
        start_line: 2,
        end_line: 2,
        source_byte_offset: Some(15),
        evidence_quote: Some("source quotation".into()),
        evidence_occurrences: Vec::new(),
        review: FactReview::Supported,
    };
    let mut current = previous.clone();
    assert!(same_source_occurrence_for_audit(&previous, &current));
    current.source_byte_offset = Some(90);
    assert!(!same_source_occurrence_for_audit(&previous, &current));
    current.source_byte_offset = Some(15);
    current.source_chunk = 1;
    assert!(!same_source_occurrence_for_audit(&previous, &current));
    current.source_chunk = 0;
    current.source_byte_offset = None;
    assert!(!same_source_occurrence_for_audit(&previous, &current));
    previous.review = FactReview::Pending;
    current.source_byte_offset = Some(15);
    assert!(!same_source_occurrence_for_audit(&previous, &current));
}

#[test]
fn repeated_statement_same_line_preserves_both_exact_offsets() {
    let source = "Type: A\nrepeat here and repeat here";
    let part = Chunk {
        start_byte: 0,
        start_line: 1,
        end_line: 2,
        text: source.into(),
    };
    let mut facts = BTreeMap::new();
    let offsets: Vec<usize> = source.match_indices("repeat").map(|(i, _)| i).collect();
    for &offset in &offsets {
        record_positioned_candidate_fact(
            &mut facts,
            "messages/upper_a.md",
            "identical assertion".into(),
            "repeat".into(),
            0,
            &part,
            Some(offset),
        );
    }
    // One published assertion, two independently located occurrences.
    let found = &facts["messages/upper_a.md"];
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].evidence_occurrences.len(), 2);
    assert_eq!(found[0].source_byte_offset, Some(offsets[0]));
    assert_eq!(
        found[0]
            .evidence_occurrences
            .iter()
            .map(|o| o.source_byte_offset)
            .collect::<Vec<_>>(),
        offsets.iter().copied().map(Some).collect::<Vec<_>>()
    );
    assert!(
        found[0]
            .evidence_occurrences
            .iter()
            .all(|o| o.start_line == 2)
    );
    record_positioned_candidate_fact(
        &mut facts,
        "messages/upper_a.md",
        "identical assertion".into(),
        "repeat".into(),
        0,
        &part,
        Some(offsets[0]),
    );
    assert_eq!(
        facts["messages/upper_a.md"][0].evidence_occurrences.len(),
        2
    );
}

#[test]
fn positioned_candidate_facts_keep_distinct_source_lines() {
    let source = "Type: A | command\n| first | value |\n| second | value |\n";
    let part = Chunk {
        start_byte: 0,
        start_line: 1,
        end_line: 3,
        text: source.into(),
    };
    let mut facts = BTreeMap::new();
    let q1 = "| first | value |";
    let q2 = "| second | value |";
    record_positioned_candidate_fact(
        &mut facts,
        "messages/upper_a.md",
        "first".into(),
        q1.into(),
        0,
        &part,
        Some(source.find(q1).unwrap()),
    );
    record_positioned_candidate_fact(
        &mut facts,
        "messages/upper_a.md",
        "second".into(),
        q2.into(),
        0,
        &part,
        Some(source.find(q2).unwrap()),
    );
    let found = &facts["messages/upper_a.md"];
    assert_eq!((found[0].start_line, found[0].end_line), (2, 2));
    assert_eq!((found[1].start_line, found[1].end_line), (3, 3));
    assert_ne!(found[0].source_byte_offset, found[1].source_byte_offset);
    assert_eq!(
        found[0].evidence_occurrences[0].source_byte_offset,
        found[0].source_byte_offset
    );
}

#[test]
fn ids_are_stable_and_preserve_unicode_source() {
    let source = "Command Q: query status\nÜnicode and evidence\n";
    let segments = evidence_segments(source);
    assert_eq!(segments[0].id, "S1");
    assert_eq!(
        segments.iter().map(|s| s.text.as_str()).collect::<String>(),
        source
    );
    assert!(segmented_source(source).contains("[S2]"));
}

#[test]
fn known_segment_is_accepted_without_model_copied_quote() {
    let source = "Command Q: query status\nA status query returns data.\n";
    let response = r#"{"files":[{"path":"messages/q.md","identity_segment_id":"S1","facts":[{"statement":"Q queries status","segment_id":"S2"}]}]}"#;
    let parsed = parse_segmented_window(response, source).unwrap();
    assert_eq!(
        parsed[0].1[0].1.as_deref(),
        Some("A status query returns data.\n")
    );
}

#[test]
fn fabricated_ids_and_ambiguous_identity_remain_unresolved() {
    let source = "Q is mentioned as an example.\n";
    let response = r#"{"files":[{"path":"messages/q.md","identity_segment_id":"S1","facts":[{"statement":"invented","segment_id":"S999"}]}]}"#;
    assert!(parse_segmented_window(response, source).is_err());
}

#[test]
fn misplaced_fact_identity_is_validated_and_promoted() {
    let source = "Type: G | Status request\nThe G message requests status.\n";
    let response = r#"{"files":[{"path":"messages/type_g.md","facts":[{"statement":"G requests status","segment_id":"S2","identity_segment_id":"S1"}]}]}"#;
    let parsed = parse_segmented_window(response, source).unwrap();
    assert_eq!(parsed[0].0, "messages/g.md");
    assert_eq!(
        parsed[0].1[0].1.as_deref(),
        Some("The G message requests status.\n")
    );
}

#[test]
fn invalid_file_does_not_discard_valid_sibling() {
    let source = "Type: G | Status request\n";
    let response = r#"{"files":[{"path":"messages/g.md","identity_segment_id":"S1","facts":[{"statement":"G status","segment_id":"S1"}]},{"path":"messages/invented.md","identity_segment_id":"S1","facts":[{"statement":"invented","segment_id":"S1"}]}]}"#;
    let (records, rejected) =
        parse_segmented_window_recoverable(response, source, source).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, "messages/g.md");
    assert_eq!(rejected, 1);
}

#[test]
fn invalid_fact_does_not_discard_valid_siblings() {
    let source = "Type: A | Alive interval\nType: B | Beam selection\n";
    let response = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Alive interval","segment_id":"S1"},{"statement":"Wrong beam","segment_id":"S2"},{"statement":"Keepalive interval","segment_id":"S1"}]}]}"#;
    let (records, rejected) =
        parse_segmented_window_recoverable(response, source, source).unwrap();
    assert_eq!(rejected, 1);
    let kept: Vec<&str> = records
        .iter()
        .flat_map(|(_, entries)| entries.iter().map(|(statement, _)| statement.as_str()))
        .collect();
    assert_eq!(kept, vec!["Alive interval", "Keepalive interval"]);
    assert!(records.iter().all(|(path, _)| path == "messages/a.md"));
}

#[test]
fn conflicting_fact_identities_are_not_salvaged_individually() {
    let source = "Type: A | Alive interval\n";
    let response = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"First","segment_id":"S1","identity_segment_id":"S1"},{"statement":"Second","segment_id":"S1","identity_segment_id":"S2"}]}]}"#;
    let (records, rejected) =
        parse_segmented_window_recoverable(response, source, source).unwrap();
    assert!(records.is_empty());
    assert_eq!(rejected, 1);
}

#[test]
fn all_invalid_files_do_not_trigger_json_recovery() {
    let source = "Type: G | Status request\n";
    let response = r#"{"files":[{"path":"messages/invented.md","identity_segment_id":"S1","facts":[{"statement":"invented","segment_id":"S1"}]}]}"#;
    let (records, rejected) =
        parse_segmented_window_recoverable(response, source, source).unwrap();
    assert!(records.is_empty());
    assert_eq!(rejected, 1);
}

#[test]
fn incomplete_json_cannot_be_salvaged() {
    let source = "Type: G | Status request\n";
    let response = r#"{"files":[{"path":"messages/g.md","facts":[{"statement":"G""#;
    assert!(parse_segmented_window_recoverable(response, source, source).is_err());
}

#[test]
fn legacy_source_anchor_is_not_trusted_as_a_quote() {
    let source = "Type: G | Status request\n";
    let response = r#"{"files":[{"path":"messages/g.md","identity_segment_id":"S1","facts":[{"statement":"G status","source_anchor":"fabricated"}]}]}"#;
    let parsed = parse_segmented_window(response, source).unwrap();
    assert_eq!(parsed[0].1[0].1, None);
}

#[test]
fn globally_declared_identity_accepts_facts_from_later_window() {
    let full_source =
        "Type: G | Status request\nOther prose.\nThe G message requests status.\n";
    let window = "The G message requests status.\n";
    let response = r#"{"files":[{"path":"messages/g.md","facts":[{"statement":"G requests status","segment_id":"S1"}]}]}"#;
    let parsed = parse_segmented_window_with_identity(response, window, full_source).unwrap();
    assert_eq!(parsed[0].0, "messages/g.md");
    assert_eq!(
        parsed[0].1[0].1.as_deref(),
        Some("The G message requests status.\n")
    );
}

#[test]
fn canonical_paths_preserve_uppercase_and_lowercase() {
    assert_eq!(
        canonical_message_path("messages/upper_a.md", "").unwrap(),
        "messages/upper_a.md"
    );
    assert_eq!(
        canonical_message_path("messages/a.md", "").unwrap(),
        "messages/a.md"
    );
    assert!(canonical_message_path("messages/upper_a.md", "# Message a").is_err());
    assert!(canonical_message_path("messages/a.md", "# Message A").is_err());
}

#[test]
fn mixed_case_facts_route_to_distinct_files() {
    let source = "Type: A | Modem alive\nType: a | Antenna alive\n";
    let response = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Type: A | Modem alive","segment_id":"S1"},{"statement":"Type: a | Antenna alive","segment_id":"S2"}]}]}"#;
    let (records, rejected) =
        parse_segmented_window_recoverable(response, source, source).unwrap();
    assert_eq!(rejected, 0);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].0, "messages/upper_a.md");
    assert_eq!(records[1].0, "messages/a.md");
}

#[test]
fn mixed_case_untyped_fact_is_not_guessed() {
    let source = "Type: A | Modem alive\nType: a | Antenna alive\nOther detail\n";
    let response = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Unattributed detail","segment_id":"S3"},{"statement":"Type: A | Modem alive","segment_id":"S1"}]}]}"#;
    let (records, rejected) =
        parse_segmented_window_recoverable(response, source, source).unwrap();
    assert_eq!(rejected, 1);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, "messages/upper_a.md");
}

#[test]
fn unrelated_lowercase_table_cell_does_not_override_uppercase_identity() {
    let source = "Type: A | Modem alive interval\n| a | unrelated parameter |\n";
    assert_eq!(message_path_type("messages/a.md", source), Some(b'A'));
    let response = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Type: A | Modem alive interval","segment_id":"S1"}]}]}"#;
    assert!(parse_segmented_window(response, source).is_ok());
}

#[test]
fn standalone_type_heading_owns_contiguous_evidence_lines() {
    let source = "Type: A\nAlive interval parameter\nModem keepalive timeout\n";
    let wrong = r#"{"files":[{"path":"messages/b.md","facts":[{"statement":"Keepalive timeout","segment_id":"S3"}]}]}"#;
    let identity_source = format!("{source}Type: B\n");
    let err =
        parse_segmented_window_with_identity(wrong, source, &identity_source).unwrap_err();
    assert!(err.contains("Evidence declares Type: A"), "{err}");
    let right = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Keepalive timeout","segment_id":"S3"}]}]}"#;
    assert!(parse_segmented_window(right, source).is_ok());
}

#[test]
fn blank_line_ends_standalone_type_ownership() {
    let source = "Type: A\nAlive interval\n\nUnrelated appendix parameter\nType: B\n";
    assert_eq!(section_type_for_line(source, 1), Some(b'A'));
    assert_eq!(section_type_for_line(source, 3), None);
}

#[test]
fn evidence_type_must_match_message_even_without_statement_type() {
    let source = "Type: A | Modem alive interval\nType: B | Beam selection\n";
    let wrong = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Beam selection","segment_id":"S2"}]}]}"#;
    let err = parse_segmented_window(wrong, source).unwrap_err();
    assert!(err.contains("Evidence declares Type: B"), "{err}");
    let right = r#"{"files":[{"path":"messages/b.md","facts":[{"statement":"Beam selection","segment_id":"S2"}]}]}"#;
    assert!(parse_segmented_window(right, source).is_ok());
}

#[test]
fn generic_evidence_does_not_invent_message_ownership() {
    let source = "Type: A | Modem alive interval\nGeneric description without type\n";
    let response = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Generic description","segment_id":"S2"}]}]}"#;
    assert!(parse_segmented_window(response, source).is_ok());
}

#[test]
fn explicitly_typed_facts_are_routed_without_losing_case() {
    let source = "Type: A | Modem alive interval\nType: a | Antenna alive interval\n";
    assert_eq!(source.lines().count(), 2);
    assert!(declared_message_identity("messages/upper_a.md", source));
    assert!(declared_message_identity("messages/a.md", source));
    let response = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Type: A | Modem alive interval","segment_id":"S1"}]}]}"#;
    let routed = parse_segmented_window(response, source).unwrap();
    assert_eq!(routed.len(), 1);
    assert_eq!(routed[0].0, "messages/upper_a.md");

    let correct = r#"{"files":[{"path":"messages/upper_a.md","facts":[{"statement":"Type: A | Modem alive interval","segment_id":"S1"}]}]}"#;
    assert_eq!(parse_segmented_window(correct, source).unwrap(), routed);

    let contradiction = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Type: a | Antenna alive interval","segment_id":"S1"}]}]}"#;
    let err = parse_segmented_window(contradiction, source).unwrap_err();
    assert!(err.contains("contradicts evidence"), "{err}");
}

#[test]
fn uppercase_and_lowercase_messages_do_not_share_identity() {
    let source = "Type: X | Modem command\nType: x | Antenna response\n";
    assert!(declared_message_identity("messages/upper_x.md", source));
    assert!(declared_message_identity("messages/x.md", source));
    assert!(!declared_message_identity("messages/upper_y.md", source));
    let only_upper = "Type: O | Modem command\n";
    assert!(declared_message_identity("messages/o.md", only_upper));
    assert!(declared_message_identity("messages/upper_o.md", only_upper));
}

#[test]
fn markdown_type_rows_declare_message_identity() {
    let source =
        "| Type | Description |\n| --- | --- |\n| O | Modem command |\n| o | Antenna reply |\n";
    assert!(declared_message_identity("messages/upper_o.md", source));
    assert!(declared_message_identity("messages/o.md", source));
}

#[test]
fn global_example_traffic_does_not_declare_identity() {
    let full_source = "Example: G 10\nThe G message requests status.\n";
    let window = "The G message requests status.\n";
    let response = r#"{"files":[{"path":"messages/g.md","facts":[{"statement":"G requests status","segment_id":"S1"}]}]}"#;
    assert!(parse_segmented_window_with_identity(response, window, full_source).is_err());
}

#[test]
fn repeated_locator_does_not_promote_unreviewed_fact() {
    let mut facts = BTreeMap::new();
    record_segmented_facts(
        &mut facts,
        "messages/g.md",
        vec![("G is a status request".into(), None)],
        0,
        1,
        1,
    );
    record_segmented_facts(
        &mut facts,
        "messages/g.md",
        vec![(
            "G is a status request".into(),
            Some("Type: G | Status request".into()),
        )],
        1,
        2,
        2,
    );
    let fact = &facts["messages/g.md"][0];
    assert_eq!(fact.review, FactReview::Pending);
    assert_eq!(fact.source_chunk, 1);
    assert_eq!(
        fact.evidence_quote.as_deref(),
        Some("Type: G | Status request")
    );
}

#[test]
fn unknown_fact_fields_are_still_rejected() {
    let source = "Type: G | Status request\n";
    let response = r#"{"files":[{"path":"messages/g.md","identity_segment_id":"S1","facts":[{"statement":"G status","segment_id":"S1","invented_field":"unsafe"}]}]}"#;
    assert!(parse_segmented_window(response, source).is_err());
}

#[test]
fn conflicting_fact_and_file_identity_is_rejected() {
    let source = "Type: G | Status request\nType: H | Other request\n";
    let response = r#"{"files":[{"path":"messages/g.md","identity_segment_id":"S1","facts":[{"statement":"G requests status","segment_id":"S1","identity_segment_id":"S2"}]}]}"#;
    assert!(parse_segmented_window(response, source).is_err());
}

#[test]
fn invented_message_identity_is_rejected() {
    let source = "Type: G | Status request\n";
    let response = r#"{"files":[{"path":"messages/type_h.md","identity_segment_id":"S1","facts":[{"statement":"H is a message","segment_id":"S1"}]}]}"#;
    assert!(parse_segmented_window(response, source).is_err());
}

#[test]
fn source_locator_is_not_a_semantic_fact_verification() {
    let mut facts = BTreeMap::new();
    record_segmented_facts(
        &mut facts,
        "messages/g.md",
        vec![(
            "G disables all safety checks".into(),
            Some("Type: G | Status request\n".into()),
        )],
        0,
        1,
        1,
    );
    assert_eq!(facts["messages/g.md"][0].review, FactReview::Pending);
}

#[test]
fn invalid_paths_are_rejected() {
    let response =
        r#"{"files":[{"path":"../escape.md","facts":[{"statement":"x","segment_id":"S1"}]}]}"#;
    assert!(parse_segmented_window(response, "source").is_err());
}
