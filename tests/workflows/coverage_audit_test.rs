use super::*;

#[test]
fn missing_locator_reasons_preserve_source_evidence() {
    use super::super::message_identity::{CandidateFact, MessageCandidate};
    let candidate = MessageCandidate {
        id: "window-0-candidate-0".into(),
        proposed_name: "unknown".into(),
        facts: [
            ("alpha", Some("alpha"), false),
            ("beta", Some("beta"), false),
            ("paraphrase", Some("alpha"), false),
            ("alpha", Some("alpha"), true),
        ]
        .into_iter()
        .map(|(statement, quote, ambiguous)| CandidateFact {
            statement: statement.into(),
            segment_id: "S1".into(),
            evidence_quote: quote.map(str::to_owned),
            source_byte_offset: Some(0),
            source_locator_ambiguous: ambiguous,
            statement_byte_offset: None,
        })
        .collect(),
    };
    let details = missing_locator_details(&[candidate], &["alpha beta alpha"]);
    let json = serde_json::to_value(&details).unwrap();
    assert_eq!(json[0]["reason"], "repeated_statement_in_window");
    assert_eq!(json[1]["reason"], "unique_statement_without_locator");
    assert_eq!(json[2]["reason"], "statement_not_verbatim_in_window");
    assert_eq!(json[3]["reason"], "ambiguous_source_locator");
    assert_eq!(json[0]["exact_quote_occurrences_in_window"], 2);
    assert_eq!(json[0]["source_byte_offset"], 0);
}

#[test]
fn coverage_window_ids_resolve_and_quotes_are_located() {
    use super::super::message_identity::{CandidateFact, MessageCandidate};
    let candidate = MessageCandidate {
        id: "coverage-window-0-candidate-2".into(),
        proposed_name: "messages/antenna.md".into(),
        facts: vec![CandidateFact {
            statement: "A paraphrased assertion".into(),
            segment_id: "S1".into(),
            evidence_quote: Some("exact evidence".into()),
            source_byte_offset: Some(7),
            source_locator_ambiguous: false,
            statement_byte_offset: None,
        }],
    };
    let details = missing_locator_details(&[candidate], &["prefix exact evidence suffix"]);
    let json = serde_json::to_value(&details).unwrap();
    assert_eq!(json[0]["source_window"], 0);
    assert_eq!(json[0]["reason"], "statement_not_verbatim_in_window");
    assert_eq!(json[0]["unique_quote_byte_offset_in_window"], 7);
    assert_eq!(json[0]["quote_at_source_byte_offset"], true);
}

#[test]
fn evidence_coverage_distinguishes_anchored_and_uncovered_lines() {
    use super::super::message_identity::{CandidateFact, MessageCandidate};
    let source = "alpha fact\nbeta fact\n";
    let candidate = MessageCandidate {
        id: "coverage-window-0-candidate-0".into(),
        proposed_name: "test".into(),
        facts: vec![CandidateFact {
            statement: "alpha paraphrased".into(),
            segment_id: "S1".into(),
            evidence_quote: Some("alpha fact".into()),
            source_byte_offset: Some(0),
            source_locator_ambiguous: false,
            statement_byte_offset: None,
        }],
    };
    let report = evidence_coverage_report(&[candidate], &[source]);
    assert_eq!(report.source_lines_considered, 2);
    assert_eq!(report.source_lines_with_evidence, 1);
    assert_eq!(report.source_lines_without_evidence, 1);
    assert_eq!(report.unanchored_facts, 0);
    assert_eq!(report.uncovered_lines[0].text, "beta fact");
    assert_eq!(report.uncovered_lines_omitted, 0);
}

#[test]
fn uncovered_inventory_limit_is_reported_explicitly() {
    let source = "Antenna sends status code 18 when blocked\n".repeat(10_001);
    let report = evidence_coverage_report(&[], &[&source]);
    assert_eq!(report.source_lines_without_evidence, 10_001);
    assert_eq!(report.uncovered_lines.len(), 10_000);
    assert_eq!(report.uncovered_lines_omitted, 1);
    let plan = semantic_review_plan(&[], &[&source], &[0], &source, &report);
    assert_eq!(plan.source_reviews_total, 10_001);
    assert_eq!(plan.source_reviews_inventory_omitted, 1);
}

#[test]
fn late_technical_lines_survive_bounded_inventory() {
    let mut source = "Index\n".repeat(10_001);
    source.push_str("The antenna must report statusCode=18 when blocked\n");
    let report = evidence_coverage_report(&[], &[&source]);
    assert_eq!(report.source_lines_without_evidence, 10_002);
    assert_eq!(report.uncovered_lines.len(), 10_000);
    assert_eq!(report.uncovered_lines_omitted, 2);
    assert!(
        report
            .uncovered_lines
            .iter()
            .any(|line| line.text.contains("statusCode=18"))
    );
    assert!(
        report
            .uncovered_lines
            .windows(2)
            .all(|pair| pair[0].line_number < pair[1].line_number)
    );
    let plan = semantic_review_plan(&[], &[&source], &[0], &source, &report);
    assert!(
        plan.source_reviews
            .iter()
            .any(|task| task.source_excerpt.contains("statusCode=18"))
    );
}

#[test]
fn review_prioritizes_short_protocol_assignments_but_not_captions() {
    assert_eq!(technical_priority("replyTo=getExtAntStatus"), 2);
    assert_eq!(technical_priority("| statusCode=18 |"), 2);
    assert_eq!(technical_priority("Table A-5. Transceiver"), 0);
    assert_eq!(technical_priority("Figure 3. Antenna"), 0);
    assert_eq!(technical_priority("Copyright 2026"), 0);
}

#[test]
fn review_selects_compact_assignment_without_prose() {
    let source = "# Protocol\n\nreplyTo=getExtAntStatus\n";
    let report = evidence_coverage_report(&[], &[source]);
    let plan = semantic_review_plan(&[], &[source], &[0], source, &report);
    assert!(
        plan.source_reviews
            .iter()
            .any(|task| task.source_excerpt.contains("replyTo=getExtAntStatus"))
    );
}

#[test]
fn review_queue_represents_late_sections_before_filling_capacity() {
    let mut source = String::from("# First section\n\n");
    for i in 0..240 {
        source.push_str(&format!(
            "The antenna must report parameter value {i} when requested.\n\n"
        ));
    }
    source.push_str("# Final section\n\nThe modem must report statusCode=18 when blocked.\n");
    let report = evidence_coverage_report(&[], &[source.as_str()]);
    let plan = semantic_review_plan(&[], &[source.as_str()], &[0], &source, &report);
    assert_eq!(plan.source_reviews.len(), 200);
    assert!(
        plan.source_reviews
            .iter()
            .any(|task| task.source_excerpt.contains("statusCode=18"))
    );
}

#[test]
fn review_uses_normalized_offsets_and_skips_navigation_and_table_headers() {
    let source = "# Table of Contents\n- Chapter 2 Protocol Specification. 3\n\n# Protocol\n| Type | Description |\n| --- | --- |\n| A | Antenna sends status code 18 when blocked |\n| B | Modem sends frequency in MHz |\n";
    let windows = [source];
    let report = evidence_coverage_report(&[], &windows);
    let plan = semantic_review_plan(&[], &windows, &[0], source, &report);
    let excerpts: Vec<_> = plan
        .source_reviews
        .iter()
        .map(|r| r.source_excerpt.as_str())
        .collect();
    assert!(
        excerpts
            .iter()
            .any(|text| text.contains("Antenna sends status code 18"))
    );
    assert!(
        excerpts
            .iter()
            .any(|text| text.contains("Modem sends frequency in MHz"))
    );
    assert!(!excerpts.iter().any(|text| text.contains("Chapter 2")));
    assert!(!excerpts.iter().any(|text| text.contains("Description")));
    assert!(excerpts.iter().all(|text| !text.contains("\\n| B |")));
}

#[test]
fn review_joins_prose_across_adjacent_windows() {
    let source = "# Protocol\nThe antenna must report the status when blocked\nand include the remaining seconds in the message.\n";
    let split = source.find("and include").unwrap();
    let windows = [&source[..split], &source[split..]];
    let report = evidence_coverage_report(&[], &windows);
    let plan = semantic_review_plan(&[], &windows, &[0, split], source, &report);
    assert!(
        plan.source_reviews
            .iter()
            .any(|task| task.source_excerpt.contains("when blocked\nand include"))
    );
    assert_eq!(plan.source_reviews.len(), 1);
}

#[test]
fn review_rejects_mismatched_window_positions() {
    let source = "# Protocol\nThe antenna must report status when blocked.\n";
    let window = "The antenna must report status when blocked.\n";
    let report = evidence_coverage_report(&[], &[window]);
    let plan = semantic_review_plan(&[], &[window], &[0], source, &report);
    assert_eq!(plan.source_reviews.len(), 0);
}

#[test]
fn global_coverage_deduplicates_overlaps_and_unions_evidence() {
    use super::super::message_identity::{CandidateFact, MessageCandidate};
    let source = "Antenna must report statusCode=18\nModem must report frequency\n";
    let candidate = MessageCandidate {
        id: "window-1-candidate-0".into(),
        proposed_name: "test".into(),
        facts: vec![CandidateFact {
            statement: "status".into(),
            segment_id: "S1".into(),
            evidence_quote: Some("Antenna must report statusCode=18".into()),
            source_byte_offset: Some(0),
            source_locator_ambiguous: false,
            statement_byte_offset: None,
        }],
    };
    let report = evidence_coverage_report_global(&[candidate], &[source, source], &[0, 0], source);
    assert_eq!(report.source_lines_considered, 2);
    assert_eq!(report.source_lines_with_evidence, 1);
    assert_eq!(report.source_lines_without_evidence, 1);
    assert_eq!(report.uncovered_lines.len(), 1);
    assert!(report.uncovered_lines[0].text.contains("frequency"));
}

#[test]
fn review_deduplicates_overlapping_windows() {
    let source = "# Protocol\nAntenna sends status code 18 when blocked\n";
    let windows = [source, source];
    let report = evidence_coverage_report(&[], &windows);
    let plan = semantic_review_plan(&[], &windows, &[0, 0], source, &report);
    assert_eq!(plan.source_reviews.len(), 1);
}

fn example() -> CoverageAudit {
    CoverageAudit::new(
        238, 235, 15, 10, 115, 238, 124, 114, 124, 0, 78, 95, 78, 13, 4,
    )
}

#[test]
fn representative_audit_counts_are_consistent() {
    let audit = example();
    assert!(audit.passed());
    let json = serde_json::to_value(&audit).unwrap();
    assert_eq!(json["statement_missing"], 114);
    assert_eq!(json["parameter_unmatched"], 13);
}

#[test]
fn detects_inconsistent_parameter_partition() {
    let audit = CoverageAudit::new(
        238, 235, 15, 10, 115, 238, 124, 114, 124, 0, 78, 95, 78, 12, 4,
    );
    assert!(!audit.passed());
    assert!(
        audit
            .checks
            .iter()
            .any(|c| c.name == "parameter_partition" && !c.passed)
    );
}

#[test]
fn detects_inconsistent_locator_partition() {
    let audit = CoverageAudit::new(
        238, 235, 15, 10, 115, 238, 124, 113, 124, 0, 78, 95, 78, 13, 4,
    );
    assert!(!audit.passed());
}

#[test]
fn assertion_context_restores_truncated_line_without_crossing_window() {
    use super::{assertion_context, verbatim_match};
    let window = "heading\nα default applies to device 1 unless overridden.\nnext";
    let offset = window.find("default").unwrap();
    let (start, excerpt) = assertion_context(window, offset, "default applies".len());
    assert_eq!(start, "heading\n".len());
    assert_eq!(excerpt, "α default applies to device 1 unless overridden.");
    assert!(verbatim_match("default applies to device 1 unless overridden.", excerpt));
    assert!(!verbatim_match("default applies to device 2", excerpt));
    assert!(!verbatim_match("default applies to device 1 unless overridden.", "default applies to"));
    assert!(!verbatim_match("", excerpt));
}

#[test]
fn assertion_validation_rejects_invented_or_missing_quotes() {
    use super::validate_assertion_judgment;
    assert!(validate_assertion_judgment("device 1", r#"{"verdict":"supported","evidence_quote":"device 2","rationale":"match"}"#).is_none());
    assert!(validate_assertion_judgment("device 1", r#"{"verdict":"contradicted","evidence_quote":"","rationale":"missing"}"#).is_none());
    assert!(validate_assertion_judgment("device 1", r#"{"verdict":"insufficient_evidence","evidence_quote":"","rationale":"condition missing"}"#).is_some());
}

#[test]
fn assertion_resolution_does_not_choose_repeated_alternatives() {
    use super::resolve_assertion_context;
    let window = "unrelated reserved value\nType: J | Reserved\n";
    let (offset, context, status) = resolve_assertion_context(window, 0, 9, "Type: J | Reserved");
    assert_eq!(offset, "unrelated reserved value\n".len());
    assert_eq!(context, "Type: J | Reserved");
    assert_eq!(status, "unique_window_statement_match");
    let repeated = format!("{window}Type: J | Reserved\n");
    assert_eq!(resolve_assertion_context(&repeated, 0, 9, "Type: J | Reserved").2,
        "ambiguous_window_statement_match");
    assert_eq!(resolve_assertion_context(window, 0, 9, "Type: K | Reserved").2,
        "anchor_only_unresolved");
}

#[test]
fn original_anchor_context_preserves_row_and_header() {
    let source = "# Section\n\n| Key | Value |\n| --- | --- |\n| mode | auto |\n| other | manual |\n";
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(source);
    let begin = source.find("mode").unwrap();
    let context = super::original_anchor_context(&analysis, begin, begin + 4, true).unwrap();
    assert_eq!(context["mapping"], "exact");
    assert!(context["table"]["row"].as_str().unwrap().contains("mode"));
    assert!(!context["table"]["row"].as_str().unwrap().contains("manual"));
    assert!(context["table"]["header"].as_str().unwrap().contains("Key"));
    assert!(super::original_anchor_context(&analysis, source.len(), source.len()+1, true).is_none());
}

#[test]
fn original_context_keeps_heading_path_and_covered_cell_origin() {
    let source = "# Outer\n\n## Inner\n\n<table><tr><td rowspan=\"2\">Owner</td><td>First</td></tr><tr><td>Second</td></tr></table>\n";
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(source);
    let begin = source.find("Second").unwrap();
    let context = super::original_anchor_context(&analysis, begin, begin + 6, true).unwrap();
    assert_eq!(context["heading_path"].as_array().unwrap().len(), 2);
    assert!(context["heading_path"][1]["text"].as_str().unwrap().contains("Inner"));
    let slots = context["table"]["logical_cells"].as_array().unwrap();
    assert_eq!(slots[0]["state"], "covered");
    assert_eq!(slots[0]["text"], "Owner");
    assert_eq!(slots[0]["origin_row"], 0);
    assert_eq!(slots[1]["state"], "explicit");
    assert_eq!(slots[1]["text"], "Second");
}

#[test]
fn relation_validation_requires_grounded_quotes_and_known_predicates() {
    let excerpt = "Mode uses integer values.";
    let relation = serde_json::json!([{
        "subject": "Mode", "subject_kind": "parameter", "predicate": "has_type", "object": "integer",
        "object_kind": "data_type", "conditions": [], "scope": "parameter", "evidence_quote": excerpt
    }]);
    assert!(super::validate_relations(excerpt, &relation.to_string()).is_some());
    let mut invalid = relation.clone();
    invalid[0]["evidence_quote"] = serde_json::json!("fabricated");
    assert!(super::validate_relations(excerpt, &invalid.to_string()).is_none());
    invalid = relation;
    invalid[0]["predicate"] = serde_json::json!("unknown");
    assert!(super::validate_relations(excerpt, &invalid.to_string()).is_none());
    assert!(super::validate_relations(excerpt, "[]").unwrap().is_empty());
}

#[test]
fn relation_signatures_reject_category_errors() {
    let excerpt = "A command has at least 0 parameters; mode is optional.";
    let base = serde_json::json!({
        "subject": "command", "subject_kind": "message",
        "predicate": "has_minimum_parameter_count", "object": "0",
        "object_kind": "count", "conditions": [], "scope": "message",
        "evidence_quote": excerpt
    });
    let valid = |r: &serde_json::Value| super::validate_relations(excerpt, &serde_json::json!([r]).to_string()).is_some();
    assert!(valid(&base));
    for (field, value) in [
        ("predicate", "has_parameter"), ("object", "-1"),
        ("scope", "original_source_utf8_bytes"), ("subject_kind", "text"),
    ] {
        let mut bad = base.clone();
        bad[field] = serde_json::json!(value);
        assert!(!valid(&bad), "{field}: {value}");
    }
    let mut optional = base;
    optional["subject"] = serde_json::json!("mode");
    optional["subject_kind"] = serde_json::json!("parameter");
    optional["predicate"] = serde_json::json!("is_optional");
    optional["object_kind"] = serde_json::json!("boolean");
    optional["object"] = serde_json::json!("true");
    assert!(valid(&optional));
    optional["predicate"] = serde_json::json!("has_type");
    assert!(!valid(&optional));
}

#[test]
fn relation_review_retains_valid_items_and_diagnoses_rejections() {
    let excerpt = "Mode uses integer values.";
    let good = serde_json::json!({
        "subject": "Mode", "subject_kind": "parameter", "predicate": "has_type",
        "object": "integer", "object_kind": "data_type", "conditions": [],
        "scope": "parameter", "evidence_quote": excerpt
    });
    let mut bad = good.clone();
    bad["evidence_quote"] = serde_json::json!("fabricated");
    let mut wrong_kind = good.clone();
    wrong_kind["object_kind"] = serde_json::json!("text");
    let review = super::review_relations(excerpt, &serde_json::json!([good, bad, wrong_kind, {}]).to_string());
    assert_eq!(review.status, "partial");
    assert_eq!(review.relations.len(), 1);
    let json = serde_json::to_value(&review).unwrap();
    assert_eq!(json["rejected"][0]["index"], 1);
    assert_eq!(json["rejected"][0]["reason"], "evidence_not_found");
    assert_eq!(json["rejected"][1]["reason"], "invalid_signature");
    assert_eq!(json["rejected"][2]["reason"], "invalid_schema");
    let malformed = super::review_relations(excerpt, "[");
    assert_eq!(malformed.status, "invalid");
    assert_eq!(serde_json::to_value(malformed).unwrap()["rejected"][0]["reason"], "invalid_json");
    assert_eq!(super::review_relations(excerpt, "[]").status, "valid");
}

#[test]
fn truncated_relation_array_recovers_only_complete_prefix() {
    let excerpt = "Mode uses integer values.";
    let good = serde_json::json!({
        "subject": "Mode", "subject_kind": "parameter", "predicate": "has_type",
        "object": "integer", "object_kind": "data_type", "conditions": [],
        "scope": "parameter", "evidence_quote": excerpt
    });
    let raw = format!("[{}, {{\"subject\":", good);
    let review = super::review_relations(excerpt, &raw);
    assert_eq!(review.status, "partial");
    assert_eq!(review.relations.len(), 1);
    assert_eq!(serde_json::to_value(review).unwrap()["rejected"][0]["reason"], "truncated_json");
    let malformed = format!("[{}, !]", good);
    assert!(super::review_relations(excerpt, &malformed).relations.is_empty());
    assert!(super::review_relations(excerpt, "[{\"subject\":").relations.is_empty());
}

#[test]
fn behavior_and_example_instances_are_separate_hypotheses() {
    let excerpt = "The controller must not send a reply until ready. Example: mode=2.";
    let behavior = serde_json::json!({
        "kind": "behavior", "actor": "controller", "action": "send a reply",
        "target": null, "trigger": null, "conditions": ["until ready"],
        "modality": "required", "polarity": "negative", "timing": null,
        "evidence_quote": "The controller must not send a reply until ready."
    });
    let instance = serde_json::json!({
        "kind": "instance", "example": "mode=2", "entity": "mode",
        "property": "value", "value": "2", "evidence_quote": "Example: mode=2."
    });
    let mut invalid = behavior.clone();
    invalid["evidence_quote"] = serde_json::json!("invented");
    let review = super::review_relations(excerpt, &serde_json::json!([behavior, instance, invalid]).to_string());
    assert_eq!(review.status, "partial");
    assert_eq!(review.behaviors.len(), 1);
    assert_eq!(review.instances.len(), 1);
    assert!(review.relations.is_empty());
}

#[test]
fn normalized_hypotheses_preserve_case_context_and_evidence() {
    use serde_json::json;
    let a = json!({"subject":"G", "predicate":"has_default", "object":"1", "evidence_quote":"first"});
    let mut b = a.clone();
    b["subject"] = json!(" G ");
    b["evidence_quote"] = json!("second");
    let mut c = a.clone();
    c["subject"] = json!("g");
    let mut d = a.clone();
    d["object"] = json!("2");
    let reviews = [
        json!({"relations":[a.clone(), c, d], "retry":{"review":{"relations":[b]}}}),
        json!({"relations":[a]}),
    ];
    let result = super::normalized_hypotheses(&reviews);
    assert_eq!(result["occurrences"], 5);
    assert_eq!(result["unique_hypotheses"], 4);
    assert_eq!(result["duplicate_occurrences"], 1);
    assert_eq!(result["potential_conflicts"].as_array().unwrap().len(), 1);
    let merged = result["groups"].as_array().unwrap().iter()
        .find(|g| g["occurrences"].as_array().unwrap().len() == 2).unwrap();
    assert_eq!(merged["occurrences"][0]["evidence_quote"], "first");
    assert_eq!(merged["occurrences"][1]["evidence_quote"], "second");
}
