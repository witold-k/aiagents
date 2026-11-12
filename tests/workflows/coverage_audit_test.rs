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
