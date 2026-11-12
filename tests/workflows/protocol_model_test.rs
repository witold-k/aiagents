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
            &[candidate("diagnosticCode=18", offset, ambiguous)],
            &["diagnosticCode=18"],
            &simplelexer::tablelexer::DocumentAnalysis::parse("diagnosticCode=18"),
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
        (
            "diagnosticCode=<int 0 to 65535>",
            "definition_syntax",
            Some("int"),
        ),
        (
            "diagnosticCode=&lt;int 0 to 65535>",
            "definition_syntax",
            Some("int"),
        ),
        ("diagnosticCode=20", "assignment_example_or_value", None),
    ] {
        let report = stage(
            &[candidate(quote, Some(0), false)],
            &[quote],
            &simplelexer::tablelexer::DocumentAnalysis::parse(quote),
        );
        let hint = &report.messages[0].parameter_hints[0];
        assert_eq!(hint.name, "diagnosticCode");
        assert_eq!(hint.syntax_kind, expected_kind);
        assert_eq!(hint.value_type.as_deref(), expected_type);
        assert!(hint.constraints.is_empty());
        assert_eq!(hint.required, None);
    }
}

#[test]
fn a_type_looking_string_without_an_exact_anchor_is_not_promoted() {
    let report = stage(
        &[candidate("diagnosticCode=<int 0 to 65535>", Some(1), false)],
        &["diagnosticCode=<int 0 to 65535>"],
        &simplelexer::tablelexer::DocumentAnalysis::parse("diagnosticCode=<int 0 to 65535>"),
    );
    assert!(report.messages[0].parameter_hints.is_empty());
}

#[test]
fn normalized_table_evidence_is_observed_without_claiming_ownership() {
    let source = "| Parameter | Meaning |\n| --- | --- |\n| diagnosticCode | status |\n";
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(source);
    let report = stage(&[], &[], &analysis);
    assert!(report.messages.is_empty());
    assert_eq!(report.table_evidence.len(), 1);
    assert!(
        report.table_evidence[0]
            .rows
            .iter()
            .any(|row| row.cells.iter().any(|cell| cell == "diagnosticCode"))
    );
}

#[test]
fn semantic_units_keep_voted_relations_without_assigning_message_identity() {
    use super::super::llm_structure::{RowEvidence, SemanticUnit, SourceGraph, StructuralRelation};
    let graph = SourceGraph {
        rows: vec![
            RowEvidence {
                line: 2,
                start: 12,
                previous: None,
                next: Some(35),
                cells: vec!["diagnosticCode".into(), "integer".into()],
            },
            RowEvidence {
                line: 3,
                start: 35,
                previous: Some(12),
                next: None,
                cells: vec!["".into(), "range 0..65535".into()],
            },
        ],
        relations: vec![StructuralRelation {
            earlier_row: 12,
            later_row: 35,
            relation: "entry_description".into(),
            votes: 2,
            valid_attempts: 3,
        }],
        assertions: vec![],
    };
    let units = vec![SemanticUnit {
        entry_row: 12,
        row_starts: vec![12, 35],
        supporting_relations: vec![0],
    }];
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse("");
    let report = stage(&[], &[], &analysis).with_semantic_units(&graph, &units, &[]);
    assert!(report.messages.is_empty());
    assert_eq!(report.semantic_units.len(), 1);
    let unit = &report.semantic_units[0];
    assert_eq!(unit.status, "proposed_voted_continuation");
    assert_eq!(unit.rows[1].cells[1], "range 0..65535");
    assert_eq!(unit.rows[1].normalized_byte_end, None);
    assert_eq!(unit.supporting_relations[0].votes, 2);
}

#[test]
fn invalid_semantic_row_reference_does_not_create_a_unit() {
    use super::super::llm_structure::{SemanticUnit, SourceGraph};
    let graph = SourceGraph {
        rows: vec![],
        relations: vec![],
        assertions: vec![],
    };
    let units = vec![SemanticUnit {
        entry_row: 10,
        row_starts: vec![10, 20],
        supporting_relations: vec![0],
    }];
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse("");
    let report = stage(&[], &[], &analysis).with_semantic_units(&graph, &units, &[]);
    assert!(report.semantic_units.is_empty());
}

#[test]
fn assembly_recognizes_explicit_declarations_in_second_cells() {
    let unit = SemanticUnitEvidence {
        entry_normalized_byte: 12,
        row_normalized_bytes: vec![12, 35],
        rows: vec![
            TableRowEvidence {
                normalized_byte_start: 12,
                normalized_byte_end: None,
                cells: vec![
                    "Timeout (timeoutSec)".into(),
                    "timeoutSec=&lt;int sec>".into(),
                ],
            },
            TableRowEvidence {
                normalized_byte_start: 35,
                normalized_byte_end: None,
                cells: vec![
                    "Diagnostic code (diagnosticCode)".into(),
                    "diagnosticCode=&lt;int 0 to 65535>".into(),
                ],
            },
        ],
        supporting_relations: vec![],
        status: "proposed_voted_continuation",
    };
    let assembled = assemble_unit(&unit);
    assert_eq!(assembled.len(), 2);
    assert_eq!(assembled[0].name, "timeoutSec");
    assert_eq!(assembled[0].type_hint.as_deref(), Some("int"));
    assert_eq!(assembled[0].unit_hint.as_deref(), Some("sec"));
    assert_eq!(assembled[0].evidence_row_starts, vec![12]);
    assert_eq!(assembled[1].name, "diagnosticCode");
    assert_eq!(assembled[1].range_hint.as_deref(), Some("0 to 65535"));
    assert_eq!(assembled[1].evidence_row_starts, vec![35]);
    assert!(assembled.iter().all(|a| a.message_owner.is_none()));
}

#[test]
fn assembly_rejects_examples_prose_and_keeps_conflicting_declarations() {
    let unit = SemanticUnitEvidence {
        entry_normalized_byte: 0,
        row_normalized_bytes: vec![0, 10],
        rows: vec![
            TableRowEvidence {
                normalized_byte_start: 0,
                normalized_byte_end: None,
                cells: vec![
                    "Parameter".into(),
                    "diagnosticCode=20".into(),
                    "diagnosticCode=&lt;int 0 to 65535>".into(),
                ],
            },
            TableRowEvidence {
                normalized_byte_start: 10,
                normalized_byte_end: None,
                cells: vec!["diagnosticCode".into(), "diagnosticCode=<string>".into()],
            },
        ],
        supporting_relations: vec![],
        status: "proposed_voted_continuation",
    };
    let assembled = assemble_unit(&unit);
    assert_eq!(assembled.len(), 2);
    assert_eq!(assembled[0].type_hint.as_deref(), Some("int"));
    assert_eq!(assembled[1].type_hint.as_deref(), Some("string"));
    assert_eq!(assembled[0].evidence_row_starts, vec![0]);
    assert_eq!(assembled[1].evidence_row_starts, vec![10]);
    assert!(declared_parameter("diagnosticCode=20").is_none());
    assert!(declared_parameter("The parameter is integer").is_none());
}

#[test]
fn unassembled_rows_retain_untyped_cells_and_continuations() {
    use super::super::llm_structure::{RowEvidence, SemanticUnit, SourceGraph};
    let graph = SourceGraph {
        rows: vec![
            RowEvidence {
                line: 346,
                start: 110836,
                previous: None,
                next: Some(110910),
                cells: vec![
                    "q Calibration frequency required for sensor.".into(),
                    "2 int sample_rate Hz".into(),
                ],
            },
            RowEvidence {
                line: 347,
                start: 110910,
                previous: Some(110836),
                next: None,
                cells: vec![
                    "Sample rate is in Hz.".into(),
                    "string Channel Used By: A, B,".into(),
                ],
            },
            RowEvidence {
                line: 368,
                start: 114576,
                previous: None,
                next: None,
                cells: vec![
                    "Timeout description".into(),
                    "timeoutSec=&lt;int sec>".into(),
                ],
            },
        ],
        relations: vec![],
        assertions: vec![],
    };
    let units = vec![
        SemanticUnit {
            entry_row: 110836,
            row_starts: vec![110836, 110910],
            supporting_relations: vec![],
        },
        SemanticUnit {
            entry_row: 114576,
            row_starts: vec![114576],
            supporting_relations: vec![],
        },
    ];
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse("");
    let report = stage(&[], &[], &analysis).with_semantic_units(&graph, &units, &[]);
    assert_eq!(report.assembled_parameters.len(), 1);
    assert_eq!(report.assembled_parameters[0].name, "timeoutSec");
    assert_eq!(report.unassembled_semantic_rows.len(), 2);
    assert_eq!(
        report.unassembled_semantic_rows[0].normalized_byte_start,
        110836
    );
    assert_eq!(
        report.unassembled_semantic_rows[1].normalized_byte_start,
        110910
    );
    assert_eq!(
        report.unassembled_semantic_rows[0].cells[1],
        "2 int sample_rate Hz"
    );
    assert!(
        report
            .unassembled_semantic_rows
            .iter()
            .all(|row| row.message_owner.is_none())
    );
}

#[test]
fn hypothetical_units_are_serialized_but_never_assembled() {
    use super::super::llm_structure::{RowEvidence, SemanticUnit, SourceGraph, StructuralRelation};
    let graph = SourceGraph {
        rows: vec![
            RowEvidence {
                line: 361,
                start: 114140,
                previous: None,
                next: Some(114234),
                cells: vec![
                    "The third parameter indicates the selected channel index".into(),
                    "int selected_channel_index".into(),
                ],
            },
            RowEvidence {
                line: 362,
                start: 114234,
                previous: Some(114140),
                next: None,
                cells: vec!["selected channel".into(), "".into()],
            },
        ],
        relations: vec![StructuralRelation {
            earlier_row: 114140,
            later_row: 114234,
            relation: "text_continuation".into(),
            votes: 2,
            valid_attempts: 3,
        }],
        assertions: vec![],
    };
    let hypothetical = vec![SemanticUnit {
        entry_row: 114140,
        row_starts: vec![114140, 114234],
        supporting_relations: vec![0],
    }];
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse("");
    let report = stage(&[], &[], &analysis).with_semantic_units(&graph, &[], &hypothetical);
    assert!(report.semantic_units.is_empty());
    assert!(report.assembled_parameters.is_empty());
    assert!(report.unassembled_semantic_rows.is_empty());
    assert_eq!(report.hypothetical_semantic_units.len(), 1);
    let unit = &report.hypothetical_semantic_units[0];
    assert_eq!(
        unit.status,
        "hypothetical_voted_boundary_without_lexical_anchor"
    );
    assert_eq!(unit.rows[0].cells[1], "int selected_channel_index");
    assert_eq!(unit.supporting_relations[0].votes, 2);
    assert_eq!(unit.supporting_relations[0].valid_attempts, 3);
    let json = serde_json::to_value(&report).expect("serialize diagnostic model");
    assert_eq!(
        json["hypothetical_semantic_units"][0]["entry_normalized_byte"],
        114140
    );
    assert_eq!(
        json["hypothetical_semantic_units"][0]["supporting_relations"][0]["relation"],
        "text_continuation"
    );
}

#[test]
fn multirow_table_retains_column_fragments_in_order() {
    use super::super::llm_structure::{RowEvidence, SemanticUnit, SourceGraph, StructuralRelation};
    let starts = [110836, 110910, 110969, 111057];
    let cells = [
        [
            "q Calibration frequency required for sensor.",
            "2 int sample_rate Hz",
        ],
        ["Sample rate is in Hz.", "string Channel Used By: A, B,"],
        [
            "Calibration required on input (A), output (B) or",
            "or both (for example,\"q 10 B\")",
        ],
        ["both (A/B)", ""],
    ];
    let rows = starts
        .iter()
        .enumerate()
        .map(|(i, start)| RowEvidence {
            line: 346 + i,
            start: *start,
            previous: i.checked_sub(1).map(|j| starts[j]),
            next: starts.get(i + 1).copied(),
            cells: cells[i].iter().map(|value| (*value).to_owned()).collect(),
        })
        .collect();
    let relations = (0..3)
        .map(|i| StructuralRelation {
            earlier_row: starts[i],
            later_row: starts[i + 1],
            relation: "text_continuation".into(),
            votes: 3,
            valid_attempts: 3,
        })
        .collect();
    let graph = SourceGraph {
        rows,
        relations,
        assertions: vec![],
    };
    let units = vec![SemanticUnit {
        entry_row: starts[0],
        row_starts: starts.to_vec(),
        supporting_relations: vec![0, 1, 2],
    }];
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse("");
    let model = stage(&[], &[], &analysis).with_semantic_units(&graph, &units, &[]);
    assert!(model.assembled_parameters.is_empty());
    assert_eq!(model.reconstructed_table_entries.len(), 1);
    let entry = &model.reconstructed_table_entries[0];
    assert_eq!(entry.row_normalized_bytes, starts);
    assert_eq!(entry.first_column_fragments[3], "both (A/B)");
    assert_eq!(
        entry.second_column_fragments[1],
        "string Channel Used By: A, B,"
    );
    assert_eq!(entry.second_column_fragments[3], "");
    assert_eq!(entry.supporting_relations.len(), 3);
    assert!(entry.message_owner.is_none());
    assert_eq!(entry.status, "voted_row_sequence_uninterpreted");
}
