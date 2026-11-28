use super::*;

fn rows() -> Vec<RowEvidence> {
    vec![
        RowEvidence {
            line: 1,
            start: 10,
            previous: None,
            next: Some(20),
            cells: vec!["first".into()],
        },
        RowEvidence {
            line: 2,
            start: 20,
            previous: Some(10),
            next: None,
            cells: vec!["second".into()],
        },
        RowEvidence {
            line: 3,
            start: 30,
            previous: None,
            next: None,
            cells: vec!["other table".into()],
        },
    ]
}

fn graph_for_projection() -> SourceGraph {
    let starts = [10, 20, 30, 40, 50, 60, 70, 80];
    let cells = [
        vec!["in degrees.", ""],
        vec![
            "q Calibration frequency required for sensor.",
            "2 int sample_rate Hz",
        ],
        vec!["Sample rate is in Hz.", "string Channel Used By: A, B,"],
        vec!["both (A/B)", ""],
        vec!["Timeout (timeoutSec)", "timeoutSec=&lt;int sec>"],
        vec!["This parameter should accompany response message 18", ""],
        vec![
            "Diagnostic code (diagnosticCode)",
            "diagnosticCode=&lt;int 0 to 65535>",
        ],
        vec!["If there are additional status messages", ""],
    ];
    let rows: Vec<_> = starts
        .iter()
        .enumerate()
        .map(|(i, start)| RowEvidence {
            line: i + 1,
            start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        })
        .collect();
    let ranked = vec![
        (10, 20, "entry_description".to_owned(), 3),
        (20, 30, "entry_description".to_owned(), 3),
        (30, 40, "text_continuation".to_owned(), 3),
        (40, 50, "independent".to_owned(), 3),
        (50, 60, "entry_description".to_owned(), 3),
        (60, 70, "independent".to_owned(), 3),
        (70, 80, "entry_description".to_owned(), 2),
        (70, 80, "text_continuation".to_owned(), 1),
    ];
    source_graph(&rows, &ranked, 3, &[])
}

#[test]
fn semantic_projection_rejects_false_previous_entry_and_keeps_named_keys() {
    let graph = graph_for_projection();
    let units = semantic_units(&graph);
    assert_eq!(units.len(), 3);
    assert_eq!(units[0].entry_row, 20);
    assert_eq!(units[0].row_starts, vec![20, 30, 40]);
    assert_eq!(units[1].entry_row, 50);
    assert_eq!(units[1].row_starts, vec![50, 60]);
    assert_eq!(units[2].entry_row, 70);
    assert_eq!(units[2].row_starts, vec![70, 80]);
    assert!(units.iter().all(|u| !u.row_starts.contains(&10)));
}

#[test]
fn unit_coverage_preserves_occurrences_and_excludes_unanchored_rows() {
    let units = semantic_units(&graph_for_projection());
    let unmatched = vec![
        (1, 10, "unclassified_row".to_owned()),
        (2, 20, "unclassified_row".to_owned()),
        (2, 20, "unclassified_row".to_owned()),
        (3, 30, "unclassified_row".to_owned()),
        (6, 60, "unclassified_row".to_owned()),
        (9, 90, "unclassified_row".to_owned()),
    ];
    let (counts, outside) = unit_fact_coverage(&units, &unmatched);
    assert_eq!(counts, vec![(20, 3), (50, 1), (70, 0)]);
    assert_eq!(outside, 2);
}

#[test]
fn semantic_projection_does_not_cross_independent_boundary() {
    let graph = graph_for_projection();
    assert!(
        semantic_units(&graph)
            .iter()
            .all(|unit| { !unit.row_starts.windows(2).any(|pair| pair == [60, 70]) })
    );
}

#[test]
fn semantic_projection_keeps_full_description_chain_and_stops_at_next_key() {
    let starts = [
        110817, 110836, 110910, 110969, 111057, 114576, 114634, 114700, 114767, 114834, 114910,
        114975,
    ];
    let cells = [
        vec!["in degrees.", ""],
        vec![
            "q Calibration frequency required for sensor.",
            "2 int sample_rate Hz",
        ],
        vec!["Sample rate is in Hz.", "string Channel Used By: A, B,"],
        vec!["Calibration required on input (A), output (B) or", "or B"],
        vec!["both (A/B)", ""],
        vec!["Timeout (timeoutSec)", "timeoutSec=&lt;int sec>"],
        vec!["This parameter should accompany response message 18", ""],
        vec!["of an impending blockage.", ""],
        vec![
            "seconds remaining before the blockage is expected to occur.",
            "",
        ],
        vec![
            "Diagnostic code (diagnosticCode)",
            "diagnosticCode=&lt;int 0 to 65535>",
        ],
        vec!["If there are additional status messages", ""],
        vec!["antenna, beyond that communicated", ""],
    ];
    let rows: Vec<_> = starts
        .iter()
        .enumerate()
        .map(|(i, start)| RowEvidence {
            line: i + 1,
            start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        })
        .collect();
    let ranked = vec![
        (110817, 110836, "entry_description".to_owned(), 3),
        (110836, 110910, "entry_description".to_owned(), 3),
        (110910, 110969, "text_continuation".to_owned(), 3),
        (110969, 111057, "text_continuation".to_owned(), 3),
        (114576, 114634, "entry_description".to_owned(), 3),
        (114634, 114700, "text_continuation".to_owned(), 3),
        (114700, 114767, "text_continuation".to_owned(), 3),
        (114767, 114834, "heading_to_entry".to_owned(), 3),
        (114834, 114910, "entry_description".to_owned(), 3),
        (114910, 114975, "text_continuation".to_owned(), 3),
    ];
    let graph = source_graph(&rows, &ranked, 3, &[]);
    let units = semantic_units(&graph);
    assert_eq!(units.len(), 3);
    assert_eq!(units[0].entry_row, 110836);
    assert_eq!(units[0].row_starts, vec![110836, 110910, 110969, 111057]);
    assert_eq!(units[1].entry_row, 114576);
    assert_eq!(units[1].row_starts, vec![114576, 114634, 114700, 114767]);
    assert_eq!(units[2].entry_row, 114834);
    assert_eq!(units[2].row_starts, vec![114834, 114910, 114975]);
    assert!(units.iter().all(|unit| !unit.row_starts.contains(&110817)));
}

#[test]
fn outside_rows_keep_headings_separate_from_reserved_boundaries() {
    let starts = [10, 20, 30, 40];
    let cells = [
        vec!["Section title", ""],
        vec!["Continuation without an entry key", "int index"],
        vec!["(Optional key value pairs)", ""],
        vec!["q Reserved", ""],
    ];
    let rows: Vec<_> = starts
        .iter()
        .enumerate()
        .map(|(i, start)| RowEvidence {
            line: i + 1,
            start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        })
        .collect();
    let graph = source_graph(
        &rows,
        &[
            (10, 20, "heading_to_entry".to_owned(), 3),
            (20, 30, "independent".to_owned(), 3),
            (30, 40, "independent".to_owned(), 3),
        ],
        3,
        &[],
    );
    let unmatched = vec![
        (2, 20, "unclassified_row".to_owned()),
        (3, 30, "possible_continuation".to_owned()),
        (4, 40, "reserved_marker_candidate".to_owned()),
    ];
    assert_eq!(
        outside_unit_evidence(&graph, &[], &unmatched),
        vec![
            (2, 20, "heading_link_without_entry_anchor"),
            (3, 30, "no_verified_unit_anchor"),
            (4, 40, "explicit_reserved_boundary"),
        ]
    );
}

#[test]
fn hypothetical_entry_requires_heading_and_continuation_without_ownership() {
    let starts = [10, 20, 30, 40, 50, 60];
    let cells = [
        vec!["Heading", ""],
        vec!["The next parameter describes an index", "int chain_index"],
        vec!["used for selecting the chain", ""],
        vec!["(Optional key value pairs)", ""],
        vec!["Named option (optionKey)", "optionKey=<int>"],
        vec!["q Reserved", ""],
    ];
    let rows: Vec<_> = starts
        .iter()
        .enumerate()
        .map(|(i, start)| RowEvidence {
            line: i + 1,
            start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        })
        .collect();
    let uncovered = vec![(6, 60, "reserved_marker_candidate".to_owned())];
    let graph = source_graph(
        &rows,
        &[
            (10, 20, "heading_to_entry".to_owned(), 3),
            (20, 30, "text_continuation".to_owned(), 3),
            (30, 40, "independent".to_owned(), 3),
            (40, 50, "heading_to_entry".to_owned(), 3),
            (50, 60, "independent".to_owned(), 3),
        ],
        3,
        &uncovered,
    );
    let verified = semantic_units(&graph);
    let units = hypothetical_units(&graph, &verified);
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].entry_row, 20);
    assert_eq!(units[0].row_starts, vec![20, 30]);
    assert!(!units[0].row_starts.contains(&40));
    assert!(!units[0].row_starts.contains(&60));
}

#[test]
fn tiered_evidence_is_disjoint_and_preserves_duplicate_occurrences() {
    let lexical = vec![SemanticUnit {
        entry_row: 10,
        row_starts: vec![10, 20],
        supporting_relations: vec![],
    }];
    let relational = vec![SemanticUnit {
        entry_row: 30,
        row_starts: vec![30, 40],
        supporting_relations: vec![],
    }];
    let evidence = vec![
        (10, true),
        (10, true),
        (20, false),
        (30, true),
        (40, false),
        (50, true),
    ];
    assert_eq!(
        tiered_unit_evidence(&lexical, &relational, &evidence),
        TieredEvidence {
            lexical_exact: 2,
            lexical_unverified: 1,
            relational_exact: 1,
            relational_unverified: 1,
            outside_or_ambiguous: 1,
        }
    );
    let overlapping = vec![SemanticUnit {
        entry_row: 30,
        row_starts: vec![10, 30],
        supporting_relations: vec![],
    }];
    assert_eq!(
        tiered_unit_evidence(&lexical, &overlapping, &[(10, true)]),
        TieredEvidence {
            outside_or_ambiguous: 1,
            ..TieredEvidence::default()
        }
    );
}

#[test]
fn remaining_evidence_distinguishes_headings_and_reserved_boundaries() {
    let starts = [10, 20, 30, 40];
    let cells = [
        vec!["(Optional key value pairs)", ""],
        vec!["Timeout (timeoutSec)", "timeoutSec=<int sec>"],
        vec!["q Reserved", ""],
        vec!["Other text", ""],
    ];
    let rows: Vec<_> = starts
        .iter()
        .enumerate()
        .map(|(i, start)| RowEvidence {
            line: i + 1,
            start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|s| (*s).to_owned()).collect(),
        })
        .collect();
    let unmatched = vec![
        (1, 10, "possible_continuation".to_owned()),
        (3, 30, "reserved_marker_candidate".to_owned()),
        (4, 40, "unclassified_row".to_owned()),
    ];
    let graph = source_graph(
        &rows,
        &[(10, 20, "heading_to_entry".to_owned(), 3)],
        3,
        &unmatched,
    );
    assert_eq!(
        remaining_unit_evidence(&graph, &[], &[], &unmatched),
        RemainingEvidence {
            structural_headings: 1,
            reserved_boundaries: 1,
            unresolved: 1,
        }
    );
    let graph_without_votes = source_graph(&rows, &[], 3, &unmatched);
    assert_eq!(
        remaining_unit_evidence(&graph_without_votes, &[], &[], &unmatched),
        RemainingEvidence {
            structural_headings: 0,
            reserved_boundaries: 1,
            unresolved: 2,
        }
    );
}

#[test]
fn only_exact_same_table_adjacency_is_valid() {
    let good = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}]}"#;
    assert_eq!(validate(good, &rows()).unwrap().accepted.len(), 1);
    let bad = r#"{"relations":[{"earlier_row_byte":20,"later_row_byte":30,"relation":"text_continuation"}]}"#;
    assert_eq!(validate(bad, &rows()).unwrap().rejected.len(), 1);
}

#[test]
fn fabricated_and_unknown_relations_are_rejected() {
    for response in [
        r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":999,"relation":"text_continuation"}]}"#,
        r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"message_identity"}]}"#,
    ] {
        assert_eq!(validate(response, &rows()).unwrap().rejected.len(), 1);
    }
}

#[test]
fn heading_and_continuation_are_distinct() {
    let heading = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"heading_to_entry"}]}"#;
    let continuation = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}]}"#;
    assert_ne!(
        validate(heading, &rows()).unwrap().accepted,
        validate(continuation, &rows()).unwrap().accepted
    );
}

#[test]
fn disagreement_is_retained_not_resolved_by_guessing() {
    let a = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}]}"#.to_owned();
    let b =
        r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"independent"}]}"#
            .to_owned();
    let RankResult {
        valid_attempts: valid,
        ranked,
        rejected,
    } = rank(&[a, b], &rows());
    assert!(rejected.is_empty());
    assert_eq!(valid, 2);
    assert_eq!(ranked.len(), 2);
    assert!(ranked.iter().all(|item| item.3 == 1));
}
#[test]
fn invalid_items_do_not_discard_valid_pairs() {
    let response = r#"{"relations":[
            {"earlier_row_byte":10,"later_row_byte":20,"relation":"heading_to_entry"},
            {"earlier_row_byte":10,"later_row_byte":null,"relation":"independent"},
            {"earlier_row_byte":10,"later_row_byte":999,"relation":"independent"},
            {"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}
        ]}"#
    .to_owned();
    let result = rank(&[response], &rows());
    assert_eq!(result.valid_attempts, 1);
    assert_eq!(result.ranked.len(), 1);
    assert_eq!(result.ranked[0].2, "heading_to_entry");
    assert_eq!(result.rejected.len(), 3);
}

#[test]
fn prompt_only_offers_real_pairs() {
    let prompt = prompt(&rows());
    assert!(prompt.contains("VALID PAIR earlier_row_byte=10 later_row_byte=20"));
    assert!(!prompt.contains("VALID PAIR earlier_row_byte=20 later_row_byte=30"));
}

#[test]
fn entry_description_is_distinct_from_sentence_continuation() {
    let description = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"entry_description"}]}"#;
    let continuation = r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"text_continuation"}]}"#;
    assert_ne!(
        validate(description, &rows()).unwrap().accepted,
        validate(continuation, &rows()).unwrap().accepted
    );
    let prompt = prompt(&rows());
    assert!(prompt.contains("'entry_description'"));
    assert!(prompt.contains("unfinished sentence"));
}

#[test]
fn uncertainty_is_valid_but_does_not_imply_ownership() {
    let uncertain =
        r#"{"relations":[{"earlier_row_byte":10,"later_row_byte":20,"relation":"uncertain"}]}"#;
    let result = validate(uncertain, &rows()).unwrap();
    assert_eq!(result.accepted.len(), 1);
    assert!(result.rejected.is_empty());
}

#[test]
fn table_role_hypothesis_requires_exact_source_cell() {
    let rows = vec![RowEvidence {
        line: 1, start: 20, previous: None, next: None,
        cells: vec!["Meaning".into(), "Opcode".into()],
    }];
    let mut proposal = TableRoleHypothesis {
        table_start: 10, role: "entity_declarations".into(),
        functions: vec![], columns: vec![],
        entity_kind: Some("message".into()), row_structure: Some("independent".into()), scope: Some("unknown".into()),
        identifier_column: Some(1), evidence_row: 20, evidence_column: 1,
        evidence_text: "Opcode".into(),
    };
    assert!(validate_table_role(&proposal, 10, &rows));
    proposal.entity_kind = Some("invented".into());
    assert!(!validate_table_role(&proposal, 10, &rows));
    proposal.entity_kind = Some("message".into());
    assert!(!validate_table_role(&proposal, 11, &rows));
    proposal.identifier_column = Some(0);
    assert!(!validate_table_role(&proposal, 10, &rows));
    proposal.identifier_column = Some(1);
    proposal.evidence_text = "Invented".into();
    assert!(!validate_table_role(&proposal, 10, &rows));
    proposal.evidence_text = "Opcode".into();
    proposal.role = "uncertain".into();
    assert!(!validate_table_role(&proposal, 10, &rows));
    proposal.identifier_column = None;
    assert!(validate_table_role(&proposal, 10, &rows));
}

#[test]
fn table_role_report_preserves_competing_hypotheses() {
    let rows = rows();
    let proposal = |role: &str| serde_json::json!({
        "table_start": 10, "role": role, "identifier_column": null,
        "evidence_row": 10, "evidence_column": 0, "evidence_text": "first"
    }).to_string();
    let report = table_role_report(10, &rows, &[proposal("properties"), proposal("enumeration"), "invalid".into()]);
    assert_eq!(report["accepted"].as_array().unwrap().len(), 2);
    assert_eq!(report["rejected"].as_array().unwrap().len(), 1);
    assert_eq!(report["semantic_verified"], false);
}

#[test]
fn table_role_report_handles_fences_and_distinguishes_rejections() {
    let rows = vec![RowEvidence {
        line: 1, start: 20, previous: None, next: None,
        cells: vec!["Opcode".into()],
    }];
    let response = serde_json::json!({
        "table_start": 10, "role": "entity_declarations",
        "identifier_column": 0, "evidence_row": 20,
        "evidence_column": 0, "evidence_text": "Opcode"
    }).to_string();
    let fenced = format!("\x60\x60\x60json\n{response}\n\x60\x60\x60");
    let bad = response.replace("Opcode", "Invented");
    let report = table_role_report(10, &rows, &[fenced, bad, "invalid".into()]);
    assert_eq!(report["accepted"].as_array().unwrap().len(), 1);
    assert_eq!(report["rejected"][0]["reason"], "invalid_source_evidence_or_role");
    assert_eq!(report["rejected"][1]["reason"], "invalid_json_or_schema");
}

#[test]
fn enumeration_and_multiple_functions_require_grounded_columns() {
    let rows = vec![RowEvidence {
        line: 1, start: 20, previous: None, next: None,
        cells: vec!["Symbol".into(), "Meaning".into()],
    }];
    let mut proposal: TableRoleHypothesis = serde_json::from_value(serde_json::json!({
        "table_start": 10, "role": "enumeration", "entity_kind": "enum_value",
        "identifier_column": 0, "evidence_row": 20, "evidence_column": 0,
        "evidence_text": "Symbol", "functions": ["declaration", "value_mapping"],
        "columns": [
            {"index": 0, "role": "identifier", "evidence_row": 20, "evidence_text": "Symbol"},
            {"index": 1, "role": "description", "evidence_row": 20, "evidence_text": "Meaning"}
        ]
    })).unwrap();
    assert!(validate_table_role(&proposal, 10, &rows));
    proposal.columns[1].evidence_row = 999;
    assert!(!validate_table_role(&proposal, 10, &rows));
    proposal.columns[1].evidence_row = 20;
    proposal.columns[1].evidence_text = "invented".into();
    assert!(!validate_table_role(&proposal, 10, &rows));
    proposal.columns[1].evidence_text = "Meaning".into();
    proposal.columns[0].role = "description".into();
    assert!(!validate_table_role(&proposal, 10, &rows));
    proposal.columns[0].role = "identifier".into();
    proposal.functions.push("declaration".into());
    assert!(!validate_table_role(&proposal, 10, &rows));
}

#[test]
fn delimiter_graph_preserves_syntax_and_competing_votes() {
    let rows = vec![
        RowEvidence { line: 1, start: 10, previous: None, next: Some(20), cells: vec!["&lt;number".into()] },
        RowEvidence { line: 2, start: 20, previous: Some(10), next: None, cells: vec!["units>".into()] },
    ];
    let evidence = serde_json::json!({"earlier_row":10,"later_row":20,"column":0,
        "earlier_cell":"&lt;number","later_cell":"units>"});
    let graph = source_graph(&rows, &[], 0, &[]);
    let report = continuation_graph_report(&graph, std::slice::from_ref(&evidence));
    assert_eq!(report["candidates"][0]["decoded_fragment"], "<number units>");
    assert_eq!(report["candidates"][0]["status"], "proposed");
    assert_eq!(report["candidates"][0]["llm_relations"], serde_json::json!([]));
    let graph = source_graph(&rows, &[(10,20,"independent".into(),2)], 3, &[]);
    let report = continuation_graph_report(&graph, std::slice::from_ref(&evidence));
    assert_eq!(report["candidates"][0]["status"], "contested");
    assert_eq!(report["candidates"][0]["llm_relations"][0]["votes"], 2);
    let mut invalid = evidence;
    invalid["later_cell"] = serde_json::json!("wrong");
    assert_eq!(continuation_graph_report(&graph, &[invalid])["candidates"], serde_json::json!([]));
}


#[test]
fn declaration_column_requires_agreement_and_exact_evidence() {
    let rows = vec![RowEvidence {
        line: 1, start: 100, previous: None, next: None,
        cells: vec!["Opcode".into(), "Meaning".into()],
    }];
    let proposal = serde_json::json!({
        "table_start": 100, "role": "entity_declarations",
        "entity_kind": "message", "row_structure": "independent",
        "identifier_column": 0, "evidence_row": 100,
        "evidence_column": 0, "evidence_text": "Opcode"
    });
    let valid = proposal.to_string();
    assert_eq!(agreed_message_declaration_column(100, &rows, &[valid.clone(), valid.clone()]), Some(0));
    assert_eq!(agreed_message_declaration_column(100, &rows, std::slice::from_ref(&valid)), None);
    for (field, value) in [
        ("entity_kind", serde_json::json!("parameter")),
        ("row_structure", serde_json::json!("mixed")),
        ("evidence_text", serde_json::json!("Invented")),
    ] {
        let mut other = proposal.clone();
        other[field] = value;
        assert_eq!(agreed_message_declaration_column(100, &rows, &[valid.clone(), other.to_string()]), None);
    }
    let mut other = proposal.clone();
    other["identifier_column"] = serde_json::json!(1);
    other["evidence_column"] = serde_json::json!(1);
    other["evidence_text"] = serde_json::json!("Meaning");
    assert_eq!(agreed_message_declaration_column(100, &rows, &[valid, other.to_string()]), None);
}


#[test]
fn classified_enum_rows_use_roles_not_headers_and_preserve_unsampled_rows() {
    let row = |start, cells: &[&str]| RowEvidence {
        line: start,
        start,
        previous: None,
        next: None,
        cells: cells.iter().map(|s| s.to_string()).collect(),
    };
    let rows = vec![
        row(10, &["Meaning", "Token"]),
        row(15, &[":---", "---:"]),
        row(20, &["Available", "A"]),
        row(30, &["Unavailable", "a"]),
        row(40, &["", "B"]),
        row(50, &["---", "---"]),
    ];
    let response = serde_json::json!({
        "table_start": 10, "role": "enumeration",
        "row_structure": "independent",
        "identifier_column": 1, "evidence_row": 20,
        "evidence_column": 1, "evidence_text": "A",
        "columns": [
            {"index": 1, "role": "identifier", "evidence_row": 20, "evidence_text": "A"},
            {"index": 0, "role": "description", "evidence_row": 20, "evidence_text": "Available"}
        ]
    });
    let responses = vec![response.to_string(), response.to_string()];
    let result = classified_enum_rows(10, &rows[..3], &rows, &responses).unwrap();
    assert_eq!(result["members"].as_array().unwrap().len(), 3);
    assert_eq!(result["members"][1]["value"], "a");
    assert_eq!(result["members"][1]["description"], "Unavailable");
    assert_eq!(result["members"][2]["value"], "---");
    assert_eq!(result["unresolved_rows"], serde_json::json!([40]));
    assert!(classified_enum_rows(10, &rows[..3], &rows, &responses[..1]).is_none());
    let mut conflicting = response.clone();
    conflicting["role"] = serde_json::json!("properties");
    assert!(classified_enum_rows(10, &rows[..3], &rows,
        &[response.to_string(), conflicting.to_string()]).is_none());
}
