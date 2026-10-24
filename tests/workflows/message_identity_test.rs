use super::*;

#[test]
fn document_context_resolves_table_type_across_chunk_boundary() {
    let document = "| Type: G | Modem message |\n| field | one |\n| field | two |\n";
    let start = document.find("| field | two |").unwrap();
    let window = &document[start..];
    let fact = CandidateFact {
        statement: "field two".into(),
        segment_id: "S1".into(),
        evidence_quote: Some("| field | two |".into()),
        source_byte_offset: Some(0),
        source_locator_ambiguous: false,
    };
    assert_eq!(
        resolve_fact_type_in_document(window, start, document, &fact),
        Ok(b'G')
    );
    assert_eq!(
        resolve_fact_type(window, &fact),
        Err(ResolutionFailure::NoSourceType)
    );
}

#[test]
fn document_context_does_not_cross_blank_table_boundary() {
    let document = "| Type: G | Modem message |\n\n| field | two |\n";
    let start = document.find("| field | two |").unwrap();
    let window = &document[start..];
    let fact = CandidateFact {
        statement: "field two".into(),
        segment_id: "S1".into(),
        evidence_quote: Some("| field | two |".into()),
        source_byte_offset: Some(0),
        source_locator_ambiguous: false,
    };
    assert_eq!(
        resolve_fact_type_in_document(window, start, document, &fact),
        Err(ResolutionFailure::NoSourceType)
    );
}

#[test]
fn document_context_rejects_mismatched_absolute_quote() {
    let document = "| Type: G | Modem message |\n| field | two |\n";
    let start = document.find("| field | two |").unwrap();
    let window = &document[start..];
    let fact = CandidateFact {
        statement: "field two".into(),
        segment_id: "S1".into(),
        evidence_quote: Some("| field | two |".into()),
        source_byte_offset: Some(0),
        source_locator_ambiguous: false,
    };
    assert_eq!(
        resolve_fact_type_in_document(window, start + 1, document, &fact),
        Err(ResolutionFailure::InvalidLocator)
    );
}

#[test]
fn unresolved_history_reports_previous_declaration_without_promoting_fact() {
    let document = "Type: G\n\n# Other section\nUnrelated statement\n";
    let start = document.find("Unrelated statement").unwrap();
    let window = &document[start..];
    let candidate = MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "messages/g.md".into(),
        facts: vec![CandidateFact {
            statement: "Unrelated statement".into(),
            segment_id: "S1".into(),
            evidence_quote: Some("Unrelated statement".into()),
            source_byte_offset: Some(0),
            source_locator_ambiguous: false,
        }],
    };
    let samples = unresolved_history_samples(&[candidate], &[window], &[start], document, 5);
    assert_eq!(samples.len(), 1);
    assert!(samples[0].contains("last_type=[line 1: Type: G]"));
    assert!(samples[0].contains("last_heading=[line 3: # Other section]"));
}

#[test]
fn flattened_message_table_history_tracks_rows_but_stops_at_next_type() {
    let document = "## 2.3 Message Types, Sender (M)\nType: N | Not Track-test mode\nNOTE: The N command is redundant.\n\nOptional antenna number\nType: W | Where interval\nGPS Time (time)\n";
    let first = document.find("Optional antenna number").unwrap();
    let second = document.find("GPS Time (time)").unwrap();
    assert_eq!(
        flattened_message_table_type(
            document,
            document[..first].bytes().filter(|b| *b == b'\n').count()
        ),
        Some(b'N')
    );
    assert_eq!(
        flattened_message_table_type(
            document,
            document[..second].bytes().filter(|b| *b == b'\n').count()
        ),
        Some(b'W')
    );
}

#[test]
fn flattened_history_never_claims_semantics_or_appendix() {
    let document = "## 2.3 Message Types, Sender (M)\nType: N | Description\n## 2.7 Semantics\nThe N command is discussed.\n# Appendix A Extended Antenna Status\n| parameter | description |\n";
    assert_eq!(flattened_message_table_type(document, 3), None);
    assert_eq!(flattened_message_table_type(document, 5), None);
}

#[test]
fn flattened_table_detection_is_independent_of_section_number() {
    let document =
        "# Protocol\n## 7.8 Message Types, Sender (Device)\nType: Q | Query\nA continuation line\n";
    assert_eq!(flattened_message_table_type(document, 3), Some(b'Q'));
}

#[test]
fn flattened_table_stops_at_any_new_heading() {
    let document = "## 7.8 Message Types\nType: Q | Query\n### Examples\nA continuation line\n";
    assert_eq!(flattened_message_table_type(document, 3), None);
}

#[test]
fn document_identity_agrees_with_local_explicit_declaration() {
    let document = "## 7.8 Message Types\nType: A | Query\nType: B | Other\n";
    let start = document.find("Type: B").unwrap();
    let window = &document[start..];
    let candidate = CandidateFact {
        statement: "Type: B | Other".into(),
        segment_id: "S1".into(),
        evidence_quote: Some("Type: B | Other".into()),
        source_byte_offset: Some(0),
        source_locator_ambiguous: false,
    };
    assert_eq!(
        resolve_fact_type_in_document(window, start, document, &candidate),
        Ok(b'B')
    );
}

#[test]
fn table_caption_boundary_accepts_arbitrary_numbering() {
    assert!(is_table_caption("Table A-2. Status fields"));
    assert!(is_table_caption("Table 17. Status fields"));
    assert!(is_table_caption("Table 2-2. Status fields"));
    assert!(!is_table_caption("Table of Contents"));
    let document =
        "## 7.8 Message Types\nType: Q | Query\nTable A-2. Status fields\nContinuation";
    assert_eq!(flattened_message_table_type(document, 3), None);
}

fn fact(statement: &str) -> CandidateFact {
    CandidateFact {
        statement: statement.into(),
        segment_id: "S1".into(),
        evidence_quote: Some("source excerpt".into()),
        source_byte_offset: None,
        source_locator_ambiguous: false,
    }
}

#[test]
fn long_table_inherits_explicit_type_beyond_twelve_rows() {
    let mut source = String::from("Type: A | command\n");
    for _ in 0..20 {
        source.push_str("| parameter | value |\n");
    }
    assert_eq!(table_block_type_for_line(&source, 20), Some(b'A'));
}

#[test]
fn table_type_does_not_cross_section_or_prose_boundaries() {
    assert_eq!(
        table_block_type_for_line("Type: A | command\n\n| other | value |", 2),
        None
    );
    assert_eq!(
        table_block_type_for_line("Type: A | command\nprose\n| other | value |", 2),
        None
    );
    assert_eq!(
        table_block_type_for_line("Type: A | command\n# Other section\n| other | value |", 2),
        None
    );
}

#[test]
fn table_type_uses_nearest_explicit_declaration_preserving_case() {
    let source = "Type: A | command\n| one | value |\nType: a | response\n| two | value |";
    assert_eq!(table_block_type_for_line(source, 1), Some(b'A'));
    assert_eq!(table_block_type_for_line(source, 3), Some(b'a'));
}

#[test]
fn resolves_fact_from_long_table_with_exact_offset() {
    let mut source = String::from("Type: A | command\n");
    for _ in 0..20 {
        source.push_str("| parameter | value |\n");
    }
    let quote = "| final | value |";
    let offset = source.len();
    source.push_str(quote);
    let candidate = MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "messages/incorrect.md".into(),
        facts: vec![CandidateFact {
            statement: "final parameter".into(),
            segment_id: "S22".into(),
            evidence_quote: Some(quote.into()),
            source_byte_offset: Some(offset),
            source_locator_ambiguous: false,
        }],
    };
    let (groups, unresolved) = resolved_source_facts(&[candidate], &[&source]);
    assert_eq!(unresolved, 0);
    assert_eq!(groups["messages/upper_a.md"].len(), 1);
}

#[test]
fn nearest_table_declaration_overrides_distant_section_heading() {
    let source = "Type: A\n| Type: B | command |\n| field | value |";
    let candidate = MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "messages/upper_a.md".into(),
        facts: vec![CandidateFact {
            statement: "field".into(),
            segment_id: "S3".into(),
            evidence_quote: Some("| field | value |".into()),
            source_byte_offset: Some(source.find("| field").unwrap()),
            source_locator_ambiguous: false,
        }],
    };
    let (groups, unresolved) = resolved_source_facts(&[candidate], &[source]);
    assert_eq!(unresolved, 0);
    assert_eq!(groups["messages/upper_b.md"].len(), 1);
}

#[test]
fn diagnostics_and_projection_agree_on_ambiguous_legacy_quote() {
    let source = "Type: A\nrepeat\nrepeat\n";
    let candidate = MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "messages/upper_a.md".into(),
        facts: vec![CandidateFact {
            statement: "parameter".into(),
            segment_id: "missing".into(),
            evidence_quote: Some("repeat".into()),
            source_byte_offset: None,
            source_locator_ambiguous: false,
        }],
    };
    let (groups, unresolved) = resolved_source_facts(&[candidate.clone()], &[source]);
    assert!(groups.is_empty());
    assert_eq!(unresolved, 1);
    let diagnostics = resolution_diagnostics(&[candidate], &[source]);
    assert_eq!(diagnostics.ambiguous_locator, 1);
}

#[test]
fn diagnoses_untyped_fact_without_promoting_it() {
    let source = "| parameter | value |";
    let candidate = MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "messages/upper_a.md".into(),
        facts: vec![CandidateFact {
            statement: "parameter".into(),
            segment_id: "S1".into(),
            evidence_quote: Some(source.into()),
            source_byte_offset: Some(0),
            source_locator_ambiguous: false,
        }],
    };
    let diagnostics = resolution_diagnostics(&[candidate], &[source]);
    assert_eq!(diagnostics.no_source_type, 1);
    assert_eq!(diagnostics.conflicting_type, 0);
}

#[test]
fn counts_duplicate_source_facts_without_merging_candidate_identities() {
    let candidates = collect_candidates(
        3,
        [
            ("messages/a.md".into(), vec![fact("shared"), fact("unique")]),
            ("messages/upper_a.md".into(), vec![fact("shared")]),
        ],
    );
    assert_eq!(distinct_source_fact_count(&candidates), 2);
    assert_eq!(candidates.len(), 2);
}

#[test]
fn consolidates_explicit_source_type_across_different_proposed_names() {
    let candidates = vec![
        MessageCandidate {
            id: "window-2-candidate-0".into(),
            proposed_name: "messages/status.md".into(),
            facts: vec![CandidateFact {
                statement: "first".into(),
                segment_id: "S1".into(),
                evidence_quote: Some("Type: A | modem status".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            }],
        },
        MessageCandidate {
            id: "coverage-window-3-candidate-0".into(),
            proposed_name: "messages/another_alias.md".into(),
            facts: vec![CandidateFact {
                statement: "second".into(),
                segment_id: "S1".into(),
                evidence_quote: Some("Type: A | modem status".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            }],
        },
    ];
    let (groups, unresolved) = resolved_source_facts(
        &candidates,
        &[
            "Type: A | modem status",
            "Type: A | modem status",
            "Type: A | modem status",
            "Type: A | modem status",
        ],
    );
    assert_eq!(unresolved, 0);
    assert_eq!(groups["messages/upper_a.md"].len(), 2);
    assert_eq!(groups["messages/upper_a.md"][0].0, 2);
    assert_eq!(groups["messages/upper_a.md"][1].0, 3);
}

#[test]
fn never_merges_upper_and_lower_or_untyped_candidates() {
    let candidates = vec![
        MessageCandidate {
            id: "window-0-candidate-0".into(),
            proposed_name: "messages/a.md".into(),
            facts: vec![CandidateFact {
                statement: "upper".into(),
                segment_id: "S1".into(),
                evidence_quote: Some("Type: A | command".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            }],
        },
        MessageCandidate {
            id: "window-0-candidate-1".into(),
            proposed_name: "messages/a.md".into(),
            facts: vec![CandidateFact {
                statement: "lower".into(),
                segment_id: "S2".into(),
                evidence_quote: Some("Type: a | report".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            }],
        },
        MessageCandidate {
            id: "window-0-candidate-2".into(),
            proposed_name: "messages/a.md".into(),
            facts: vec![fact("unknown")],
        },
    ];
    let (groups, unresolved) = resolved_source_facts(
        &candidates,
        &["Type: A | command\nType: a | report\n\nsource excerpt"],
    );
    assert_eq!(groups["messages/upper_a.md"].len(), 1);
    assert_eq!(groups["messages/a.md"].len(), 1);
    assert_eq!(unresolved, 1);
}

#[test]
fn source_section_resolves_untyped_fact_independently_of_candidate_name() {
    let source =
        "Type: A\nfield one is required\nfield two is optional\n\nType: a\nfield three is required";
    let candidates = vec![MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "messages/unknown.md".into(),
        facts: vec![
            CandidateFact {
                statement: "first".into(),
                segment_id: "S2".into(),
                evidence_quote: Some("field one is required".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            },
            CandidateFact {
                statement: "second".into(),
                segment_id: "S5".into(),
                evidence_quote: Some("field three is required".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            },
        ],
    }];
    let (groups, unresolved) = resolved_source_facts(&candidates, &[source]);
    assert_eq!(unresolved, 0);
    assert_eq!(groups["messages/upper_a.md"].len(), 1);
    assert_eq!(groups["messages/a.md"].len(), 1);
}

#[test]
fn recovery_window_stores_parent_byte_offset() {
    let parent = "Präambel 🛰️\nType: A\nshared field\n\nType: a\nshared field\n";
    let window = "Type: a\nshared field\n";
    let response = r#"{"files":[{"path":"messages/anything.md","facts":[{"statement":"lower field","segment_id":"S2"}]}]}"#;
    let candidates = parse_candidates_in_parent(response, window, parent, 0, 0);
    assert_eq!(candidates.len(), 1);
    let fact = &candidates[0].facts[0];
    let expected = parent.rfind("shared field").unwrap();
    assert_eq!(fact.source_byte_offset, Some(expected));
    assert!(!fact.source_locator_ambiguous);
    let (groups, unresolved) = resolved_source_facts(&candidates, &[parent]);
    assert_eq!(unresolved, 0);
    assert_eq!(groups["messages/a.md"].len(), 1);
}

#[test]
fn repeated_recovery_window_never_guesses_parent_location() {
    let window = "Type: A\nshared field\n";
    let parent = format!("{window}middle\n{window}");
    let response = r#"{"files":[{"path":"messages/anything.md","facts":[{"statement":"shared field","segment_id":"S2"}]}]}"#;
    let candidates = parse_candidates_in_parent(response, window, &parent, 0, 0);
    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].facts[0].source_locator_ambiguous);
    let (groups, unresolved) = resolved_source_facts(&candidates, &[&parent]);
    assert!(groups.is_empty());
    assert_eq!(unresolved, 1);
}

#[test]
fn segment_locator_disambiguates_repeated_source_text() {
    let source = "Type: A\nshared field\n\nType: a\nshared field";
    let candidates = vec![MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "messages/alias.md".into(),
        facts: vec![
            CandidateFact {
                statement: "upper shared field".into(),
                segment_id: "S2".into(),
                evidence_quote: Some("shared field".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            },
            CandidateFact {
                statement: "lower shared field".into(),
                segment_id: "S4".into(),
                evidence_quote: Some("shared field".into()),
                source_byte_offset: None,
                source_locator_ambiguous: false,
            },
        ],
    }];
    let (groups, unresolved) = resolved_source_facts(&candidates, &[source]);
    assert_eq!(unresolved, 0);
    assert_eq!(groups["messages/upper_a.md"].len(), 1);
    assert_eq!(groups["messages/a.md"].len(), 1);
}

#[test]
fn conflicting_repeated_quote_stays_unresolved() {
    let source = "Type: A\nshared field\n\nType: a\nshared field";
    let candidates = vec![MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "messages/a.md".into(),
        facts: vec![CandidateFact {
            statement: "shared field".into(),
            segment_id: "S99".into(),
            evidence_quote: Some("shared field".into()),
            source_byte_offset: None,
            source_locator_ambiguous: false,
        }],
    }];
    let (groups, unresolved) = resolved_source_facts(&candidates, &[source]);
    assert!(groups.is_empty());
    assert_eq!(unresolved, 1);
}

#[test]
fn same_label_does_not_collapse_distinct_candidates() {
    let candidates = collect_candidates(
        3,
        [
            ("a".into(), vec![fact("antenna request")]),
            ("a".into(), vec![fact("modem interval")]),
        ],
    );
    assert_eq!(candidates.len(), 2);
    assert_ne!(candidates[0].id, candidates[1].id);
    assert_eq!(candidates[0].proposed_name, candidates[1].proposed_name);
}

#[test]
fn arbitrary_free_text_names_survive_collection() {
    let label = "Get extant status / antenna state (legacy name)";
    let candidates = collect_candidates(8, [(label.into(), vec![fact("status")])]);
    assert_eq!(candidates[0].proposed_name, label);
    assert_eq!(candidates[0].facts[0].segment_id, "S1");
}

#[test]
fn case_distinct_types_are_not_merged_during_collection() {
    let candidates = collect_candidates(
        2,
        [
            ("A".into(), vec![fact("modem status interval")]),
            ("a".into(), vec![fact("antenna L interval")]),
        ],
    );
    assert_eq!(candidates.len(), 2);
    assert_ne!(candidates[0].proposed_name, candidates[1].proposed_name);
}

#[test]
fn candidate_roundtrip_preserves_original_names_and_evidence() {
    let candidates =
        collect_candidates(5, [("Antenna Status".into(), vec![fact("status report")])]);
    let json = serde_json::to_string(&candidates).unwrap();
    let restored: Vec<MessageCandidate> = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, candidates);
}

#[test]
fn stages_unknown_descriptive_identity_with_source_evidence() {
    let source = "Antenna status reports the active satellite.";
    let response = r#"{"files":[{"path":"messages/antenna_status.md","facts":[{"statement":"Reports the active satellite","segment_id":"S1"}]}]}"#;
    let candidates = parse_candidates(response, source, 7, 0);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].proposed_name, "messages/antenna_status.md");
    assert_eq!(
        candidates[0].facts[0].evidence_quote.as_deref(),
        Some(source)
    );
}

#[test]
fn staging_rejects_fabricated_locators_and_type_contradictions() {
    let source = "Type: A | Modem alive";
    let response = r#"{"files":[{"path":"messages/custom.md","facts":[{"statement":"Type: B | Beam","segment_id":"S1"},{"statement":"Valid fact","segment_id":"S999"},{"statement":"Type: A | Modem alive","segment_id":"S1"}]}]}"#;
    let candidates = parse_candidates(response, source, 1, 0);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].facts.len(), 1);
    assert_eq!(candidates[0].facts[0].statement, "Type: A | Modem alive");
}

#[test]
fn classification_never_trusts_proposed_filename() {
    let candidate = MessageCandidate {
        id: "one".into(),
        proposed_name: "messages/a.md".into(),
        facts: vec![CandidateFact {
            statement: "Antenna message".into(),
            segment_id: "S1".into(),
            evidence_quote: Some("Type: B | Source declaration".into()),
            source_byte_offset: None,
            source_locator_ambiguous: false,
        }],
    };
    assert_eq!(
        classify_candidate(&candidate),
        CandidateIdentity::Resolved("messages/upper_b.md".into())
    );
}

#[test]
fn classification_keeps_unknown_and_conflicting_types_unresolved() {
    let mut candidate = MessageCandidate {
        id: "one".into(),
        proposed_name: "messages/a.md".into(),
        facts: vec![fact("no source type")],
    };
    assert_eq!(
        classify_candidate(&candidate),
        CandidateIdentity::Unresolved
    );
    candidate.facts[0].evidence_quote = Some("Type: A | one".into());
    candidate.facts.push(CandidateFact {
        statement: "two".into(),
        segment_id: "S2".into(),
        evidence_quote: Some("Type: a | two".into()),
        source_byte_offset: None,
        source_locator_ambiguous: false,
    });
    assert_eq!(
        classify_candidate(&candidate),
        CandidateIdentity::Unresolved
    );
}

#[test]
fn malformed_json_is_not_staged() {
    let candidates = parse_candidates(r#"{"files":["#, "source", 0, 0);
    assert!(candidates.is_empty());
}

#[test]
fn identity_decisions_are_explicit_and_auditable() {
    let decision = IdentityDecision::Unresolved {
        left: "window-1-candidate-0".into(),
        right: "window-2-candidate-0".into(),
        reason: "same label without identity evidence".into(),
    };
    let json = serde_json::to_string(&decision).unwrap();
    assert!(json.contains("unresolved"));
    assert!(json.contains("reason"));
    let merge = IdentityDecision::Merge {
        left: "a".into(),
        right: "b".into(),
        reason: "explicit source equivalence".into(),
    };
    let split = IdentityDecision::Split {
        left: "A".into(),
        right: "a".into(),
        reason: "different source-declared directions".into(),
    };
    assert!(serde_json::to_string(&merge).unwrap().contains("merge"));
    assert!(serde_json::to_string(&split).unwrap().contains("split"));
}
