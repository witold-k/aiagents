use super::*;

mod checkpoint_review_tests {
    use super::*;

    #[test]
    fn legacy_checkpoint_without_warning_field_is_unverified() {
        let checkpoint: ExtractionCheckpoint = serde_json::from_str(
            r#"{"version":5,"source_fingerprint":"test","next_chunk":0,"files":{},"source_windows":{},"coverage_next_chunk":0,"audit":null}"#,
        )
        .unwrap();
        assert!(checkpoint.unresolved_findings);
    }

    #[test]
    fn explicitly_clean_checkpoint_stays_clean() {
        let checkpoint: ExtractionCheckpoint = serde_json::from_str(
            r#"{"version":5,"source_fingerprint":"test","next_chunk":0,"files":{},"source_windows":{},"coverage_next_chunk":0,"unresolved_findings":false,"audit":null}"#,
        )
        .unwrap();
        assert!(!checkpoint.unresolved_findings);
    }

    #[test]
    fn detects_legacy_internal_envelopes() {
        assert!(internal_workflow_marker(
            "=== CONSOLIDATED SPECIFICATION ==="
        ));
        assert!(!internal_workflow_marker("# General protocol rules"));
    }
}

mod structured_extraction_tests {
    use super::*;

    #[test]
    fn source_quotes_are_checked_before_accepting_json_facts() {
        let source = "The command timeout is exactly thirty seconds.";
        let good = r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Timeout is thirty seconds.","evidence_quote":"timeout is exactly thirty seconds"}]}]}"#;
        let records = parse_structured_window(good, source).unwrap();
        assert_eq!(records[0].0, "messages/a.md");
        let mut facts = BTreeMap::new();
        record_verified_facts(&mut facts, &records[0].0, records[0].1.clone(), 0, 1, 1);
        assert_eq!(facts["messages/a.md"][0].review, FactReview::Supported);
        let unsupported = parse_structured_window(
            r#"{"files":[{"path":"messages/a.md","facts":[{"statement":"Invented","evidence_quote":"this is not in the source"}]}]}"#,
            source
        )
        .unwrap();
        assert_eq!(unsupported_quote_count(&unsupported), 1);
        record_verified_facts(
            &mut facts,
            &unsupported[0].0,
            unsupported[0].1.clone(),
            0,
            1,
            1,
        );
        assert_eq!(facts["messages/a.md"][1].review, FactReview::Unresolved);
        assert_eq!(facts["messages/a.md"][1].evidence_quote, None);
        assert!(parse_structured_window(
            r#"{"files":[{"path":"../bad.md","facts":[{"statement":"Invented","evidence_quote":"timeout is exactly thirty seconds"}]}]}"#,
            source
        ).is_err());
        assert!(
            parse_structured_window(r#"{"files":[]}"#, source)
                .unwrap()
                .is_empty()
        );
    }
}

mod coverage_json_tests {
    use super::*;

    #[test]
    fn coverage_accepts_structured_message_facts() {
        let source = "Message s reports antenna status to the modem.";
        let response = r#"{"files":[{"path":"messages/s_message.md","facts":[{"statement":"Message s reports antenna status","source_anchor":"Message s reports antenna status"}]}]}"#;
        let records = parse_structured_window(response, source).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(unsupported_quote_count(&records), 0);
        assert_eq!(
            canonical_message_path(&records[0].0, &records[0].1[0].0),
            "messages/s.md"
        );
    }

    #[test]
    fn arbitrary_descriptive_message_names_are_not_collapsed() {
        assert_eq!(
            canonical_message_path("messages/antenna_test_mode.md", "Antenna test mode"),
            "messages/antenna_test_mode.md"
        );
        assert_eq!(
            canonical_message_path("messages/w_message.md", "Platform location"),
            "messages/w.md"
        );
    }
}

mod partial_extraction_tests {
    use super::*;

    #[test]
    fn malformed_ebnf_quote_does_not_discard_valid_fact() {
        let source =
            "The timeout is exactly thirty seconds.\n<msg> ::= <msg_body> | <comment_body>";
        let response = r#"{"files":[{"path":"general.md","facts":[{"statement":"Timeout is thirty seconds","evidence_quote":"timeout is exactly thirty seconds"},{"statement":"Grammar contains message body and comment","evidence_quote":"<msg>::=<msg_body>|<comment_body>"}]}]}"#;
        let records = parse_structured_window(response, source).unwrap();
        assert_eq!(unsupported_quote_count(&records), 1);
        let mut facts = BTreeMap::new();
        record_verified_facts(
            &mut facts,
            &records[0].0,
            records[0].1.clone(),
            22,
            159,
            193,
        );
        assert_eq!(facts["general.md"].len(), 2);
        assert_eq!(facts["general.md"][0].review, FactReview::Supported);
        assert_eq!(facts["general.md"][1].review, FactReview::Unresolved);
        assert_eq!(facts["general.md"][1].source_chunk, 22);
        assert!(facts["general.md"][1].evidence_quote.is_none());
    }

    #[test]
    fn source_anchor_copies_original_source_bytes() {
        let source = "The modem sends <msg> ::= <body> | <comment> exactly.";
        let response = r#"{"files":[{"path":"general.md","facts":[{"statement":"Message grammar is defined","source_anchor":"<msg> ::= <body> | <comment>"}]}]}"#;
        let records = parse_structured_window(response, source).unwrap();
        assert_eq!(
            records[0].1[0].1.as_deref(),
            Some("<msg> ::= <body> | <comment>")
        );
    }

    #[test]
    fn repeated_source_anchor_is_unresolved() {
        let source = "Same source anchor text. Same source anchor text.";
        let response = r#"{"files":[{"path":"general.md","facts":[{"statement":"An assertion","source_anchor":"Same source anchor text"}]}]}"#;
        let records = parse_structured_window(response, source).unwrap();
        assert_eq!(unsupported_quote_count(&records), 1);
    }

    #[test]
    fn legacy_evidence_quote_still_works() {
        let source = "The command timeout is exactly thirty seconds.";
        let response = r#"{"files":[{"path":"general.md","facts":[{"statement":"Timeout is thirty seconds","evidence_quote":"timeout is exactly thirty seconds"}]}]}"#;
        let records = parse_structured_window(response, source).unwrap();
        assert_eq!(unsupported_quote_count(&records), 0);
    }

    #[test]
    fn supported_retry_can_upgrade_unresolved_fact() {
        let mut facts = BTreeMap::new();
        record_verified_facts(
            &mut facts,
            "general.md",
            vec![("Message grammar".to_owned(), None)],
            22,
            159,
            193,
        );
        record_verified_facts(
            &mut facts,
            "general.md",
            vec![(
                "Message grammar".to_owned(),
                Some("<msg> ::= <msg_body> | <comment_body>".to_owned()),
            )],
            22,
            159,
            193,
        );
        assert_eq!(facts["general.md"].len(), 1);
        assert_eq!(facts["general.md"][0].review, FactReview::Supported);
        assert!(facts["general.md"][0].evidence_quote.is_some());
    }
}

mod json_output_tests {
    use super::*;

    #[test]
    fn json_output_preserves_facts_and_creates_manifest() {
        let root = std::env::temp_dir().join(format!(
            "aiagents-json-output-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let output = root.join("result");
        let mut files = BTreeMap::new();
        files.insert("general.md".to_owned(), "# General".to_owned());
        files.insert("messages/b.md".to_owned(), "# Message B".to_owned());
        let mut facts = BTreeMap::new();
        record_source_facts(&mut facts, "general.md", "# General", 0, 1, 2);
        record_source_facts(&mut facts, "messages/b.md", "# Message B", 1, 3, 4);
        write_spec_directory(
            &output,
            &files,
            Path::new("source.md"),
            true,
            &BTreeMap::new(),
            &facts,
        )
        .unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["verification"], "unverified");
        assert_eq!(manifest["files"][0], "general.json");
        assert_eq!(manifest["files"][1], "messages/b.json");
        let spec: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("messages/b.json")).unwrap()).unwrap();
        assert_eq!(spec["facts"][0]["statement"], "# Message B");
        assert_eq!(spec["facts"][0]["source_chunk"], 1);
        assert_eq!(spec["verification"], "unresolved");
        assert!(output.join("facts.json").exists());
        assert!(!output.join("general.md").exists());
        assert!(!output.join("README.md").exists());
        assert!(
            write_spec_directory(
                &output,
                &files,
                Path::new("source.md"),
                true,
                &BTreeMap::new(),
                &facts,
            )
            .is_err()
        );
        fs::remove_dir_all(&root).unwrap();
    }
}

mod structured_fact_tests {
    use super::*;

    #[test]
    fn repeated_facts_are_deduplicated_with_first_source_retained() {
        let mut facts = BTreeMap::new();
        record_source_facts(&mut facts, "general.md", "Rule A.\n\nRule B.", 2, 10, 15);
        record_source_facts(&mut facts, "general.md", "Rule A.\n\nRule C.", 3, 14, 20);
        let entries = &facts["general.md"];
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].source_chunk, 2);
        assert_eq!(entries[0].start_line, 10);
        assert_eq!(
            render_source_facts(entries),
            "Rule A.\n\nRule B.\n\nRule C."
        );
    }

    #[test]
    fn canonical_identity_requires_explicit_command_heading() {
        assert_eq!(
            canonical_message_path("messages/keepalive.md", "# Command A\n\nDescription"),
            "messages/a.md"
        );
        assert_eq!(
            canonical_message_path("messages/beat_frequency.md", "# Message B\n\nDescription"),
            "messages/b.md"
        );
        assert_eq!(
            canonical_message_path("messages/keepalive.md", "# Keepalive\n\nDescription"),
            "messages/keepalive.md"
        );
        assert_eq!(
            canonical_message_path("general.md", "# Command A"),
            "general.md"
        );
    }

    #[test]
    fn fact_evidence_requires_verbatim_source_quote() {
        let source = "The timeout is exactly 30 seconds in this mode.";
        assert_eq!(
            verified_fact_quote(
                r#"{"evidence_quote":"timeout is exactly 30 seconds"}"#,
                source,
            )
            .unwrap(),
            "timeout is exactly 30 seconds"
        );
        assert!(
            verified_fact_quote(
                r#"{"evidence_quote":"timeout is exactly 60 seconds"}"#,
                source,
            )
            .is_err()
        );
        assert!(verified_fact_quote(r#"{"evidence_quote":"timeout"}"#, source).is_err());
        assert!(verified_fact_quote("UNSUPPORTED", source).is_err());
    }

    #[test]
    fn legacy_fact_review_defaults_to_pending() {
        let fact: SourceFact = serde_json::from_str(
            r#"{"statement":"A","source_chunk":1,"start_line":2,"end_line":3}"#,
        )
        .unwrap();
        assert_eq!(fact.review, FactReview::Pending);
        assert_eq!(fact.evidence_quote, None);
    }

    #[test]
    fn contradictory_facts_are_preserved_for_review() {
        let mut facts = BTreeMap::new();
        record_source_facts(&mut facts, "general.md", "Timeout is zero.", 0, 1, 3);
        record_source_facts(&mut facts, "general.md", "Timeout is nonzero.", 1, 4, 6);
        assert_eq!(facts["general.md"].len(), 2);
    }
}
