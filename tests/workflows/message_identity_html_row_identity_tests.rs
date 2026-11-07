use super::*;

#[test]
fn separate_messages_on_one_html_line_keep_case() {
    let document = concat!(
        "## Message Types\n",
        "<table><tr><td>Type</td><td>Description</td></tr>",
        "<tr><td>B</td><td>upper</td></tr>",
        "<tr><td>b</td><td>lower</td></tr></table>"
    );
    let structure = SourceStructure::from_document(document);
    for (letter, description) in [(b'B', "upper"), (b'b', "lower")] {
        let offset = document.find(description).unwrap();
        let fact = CandidateFact {
            statement: description.to_owned(),
            segment_id: String::new(),
            evidence_quote: Some(description.to_owned()),
            source_byte_offset: Some(offset),
            source_locator_ambiguous: false,
            statement_byte_offset: None,
        };
        assert_eq!(
            resolve_fact_type_with_flattened_tables(document, 0, document, &fact, true, &structure),
            Ok(letter)
        );
    }
}

#[test]
fn normalized_rows_keep_original_case_and_table_free_fallback() {
    let original = concat!(
        "## Message Types\n",
        "<table><tr><td>Type</td><td>Description</td></tr>",
        "<tr><td>B</td><td>upper evidence</td></tr>",
        "<tr><td>b</td><td>lower evidence</td></tr></table>\n",
        "## Other\nplain paragraph"
    );
    let normalized_tables = simplelexer::tablelexer::normalize_table_text(original);
    let normalized = normalized_tables.text;
    let spans = normalized_tables.rows;
    let original_analysis = simplelexer::tablelexer::DocumentAnalysis::parse(original);
    let normalized_analysis = simplelexer::tablelexer::DocumentAnalysis::parse(&normalized);
    let structure = SourceStructure::from_analysis(&normalized, &normalized_analysis)
        .with_original_table_rows(original, original, &original_analysis, &spans);
    for (letter, evidence) in [(b'B', "upper evidence"), (b'b', "lower evidence")] {
        let offset = normalized.find(evidence).unwrap();
        assert_eq!(structure.table_row_type_at(offset), Some(letter));
    }
    assert_eq!(
        structure.table_row_type_at(normalized.find("plain paragraph").unwrap()),
        None
    );
    let table_free = "## Message Types\nplain paragraph";
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(table_free);
    let fallback = SourceStructure::from_analysis(table_free, &analysis).with_original_table_rows(
        table_free,
        table_free,
        &analysis,
        &[],
    );
    assert_eq!(
        fallback.table_row_type_at(table_free.find("paragraph").unwrap()),
        None
    );
}

#[test]
fn fabricated_partial_source_row_mapping_cannot_claim_identity() {
    let original = concat!(
        "## Message Types\n",
        "<table><tr><td>Type</td><td>Description</td></tr>",
        "<tr><td>B</td><td>typed evidence</td></tr></table>"
    );
    let normalized = original.to_owned();
    let analysis = simplelexer::tablelexer::DocumentAnalysis::parse(original);
    let start = original.find("typed evidence").unwrap();
    let fake = simplelexer::tablelexer::RowTextMapping {
        source: start..start + "typed evidence".len(),
        rendered: start..start + "typed evidence".len(),
    };
    let structure = SourceStructure::from_analysis(&normalized, &analysis)
        .with_original_table_rows(original, original, &analysis, &[fake]);
    // The original parsed row still owns its type; the fabricated
    // partial mapping must not introduce a second row or owner.
    assert_eq!(structure.table_row_ranges.len(), 2);
    assert_eq!(structure.table_row_owners.len(), 1);
    assert_eq!(structure.table_row_type_at(start), Some(b'B'));
}

#[test]
fn empty_html_type_cell_does_not_panic_or_claim_identity() {
    let document = concat!(
        "## Message Types\n",
        "<table><tr><td>Type</td><td>Description</td></tr>",
        "<tr><td></td><td>untyped evidence</td></tr>",
        "<tr><td>B</td><td>typed evidence</td></tr></table>"
    );
    // Parsing the original HTML exercises the empty-cell path that
    // previously panicked; positive row ownership is covered separately.
    let structure = SourceStructure::from_document(document);
    assert_eq!(
        structure.table_row_type_at(document.find("untyped evidence").unwrap()),
        None
    );
}

#[test]
fn untyped_html_row_blocks_line_fallback_after_typed_row() {
    let document = concat!(
        "## Message Types\n",
        "<table><tr><td>Type</td><td>Description</td></tr>",
        "<tr><td>B</td><td>typed evidence</td></tr>",
        "<tr><td></td><td>untyped evidence</td></tr></table>"
    );
    let structure = SourceStructure::from_document(document);
    let offset = document.find("untyped evidence").unwrap();
    let fact = CandidateFact {
        statement: "untyped evidence".to_owned(),
        segment_id: String::new(),
        evidence_quote: Some("untyped evidence".to_owned()),
        source_byte_offset: Some(offset),
        source_locator_ambiguous: false,
        statement_byte_offset: None,
    };
    assert!(structure.is_message_table_row_at(offset));
    assert_eq!(
        resolve_fact_type_with_flattened_tables(document, 0, document, &fact, true, &structure),
        Err(ResolutionFailure::NoSourceType)
    );
}

#[test]
fn normalized_untyped_html_row_remains_a_boundary() {
    let original = concat!(
        "## Message Types\n",
        "<table><tr><td>Type</td><td>Description</td></tr>",
        "<tr><td>B</td><td>typed evidence</td></tr>",
        "<tr><td></td><td>untyped evidence</td></tr></table>"
    );
    let normalized_tables = simplelexer::tablelexer::normalize_table_text(original);
    let normalized = normalized_tables.text;
    let spans = normalized_tables.rows;
    let original_analysis = simplelexer::tablelexer::DocumentAnalysis::parse(original);
    let normalized_analysis = simplelexer::tablelexer::DocumentAnalysis::parse(&normalized);
    let structure = SourceStructure::from_analysis(&normalized, &normalized_analysis)
        .with_original_table_rows(original, original, &original_analysis, &spans);
    let offset = normalized.find("untyped evidence").unwrap();
    assert!(structure.is_message_table_row_at(offset));
    assert_eq!(structure.table_row_type_at(offset), None);
    assert!(!structure.is_message_table_row_at(normalized.len()));
}

#[test]
fn adjacent_html_tables_do_not_share_row_ownership() {
    let document = concat!(
        "## Message Types\n",
        "<table><tr><td>Type</td><td>Description</td></tr>",
        "<tr><td>B</td><td>first message</td></tr></table>\n",
        "<table><tr><td>Type</td><td>Description</td></tr>",
        "<tr><td></td><td>second untyped</td></tr></table>\n",
        "## Other\nordinary prose"
    );
    let structure = SourceStructure::from_document(document);
    let typed = document.find("first message").unwrap();
    let untyped = document.find("second untyped").unwrap();
    let prose = document.find("ordinary prose").unwrap();
    assert_eq!(structure.table_row_type_at(typed), Some(b'B'));
    assert!(structure.is_message_table_row_at(untyped));
    assert_eq!(structure.table_row_type_at(untyped), None);
    assert!(!structure.is_message_table_row_at(prose));
    assert_eq!(structure.table_row_type_at(prose), None);
}

#[test]
fn html_row_with_conflicting_fact_type_is_rejected() {
    let document = concat!(
        "## Message Types\n",
        "<table><tr><td>Type</td><td>Description</td></tr>",
        "<tr><td>B</td><td>typed evidence</td></tr></table>"
    );
    let structure = SourceStructure::from_document(document);
    let offset = document.find("typed evidence").unwrap();
    let fact = CandidateFact {
        statement: "Type: b | typed evidence".to_owned(),
        segment_id: String::new(),
        evidence_quote: Some("typed evidence".to_owned()),
        source_byte_offset: Some(offset),
        source_locator_ambiguous: false,
        statement_byte_offset: None,
    };
    assert_eq!(
        resolve_fact_type_with_flattened_tables(document, 0, document, &fact, true, &structure),
        Err(ResolutionFailure::ConflictingType)
    );
}

#[test]
fn description_rowspan_never_inherits_message_type() {
    // A rowspan in the Description column is structural grouping, not
    // evidence that an empty Type cell belongs to the previous message.
    let document = concat!(
        "## Message Types\n",
        "<table><tr><td>Type</td><td>Description</td><td>Parameters</td></tr>",
        "<tr><td>y</td><td>typed response</td><td>replyTo=getConfigB</td></tr>",
        "<tr><td></td><td rowspan=\"3\">getConfigB response</td><td>first detail</td></tr>",
        "<tr><td></td><td>second detail</td></tr>",
        "<tr><td></td><td>third detail</td></tr></table>"
    );
    let structure = SourceStructure::from_document(document);
    assert_eq!(
        structure.table_row_type_at(document.find("typed response").unwrap()),
        Some(b'y')
    );
    for evidence in ["first detail", "second detail", "third detail"] {
        let offset = document.find(evidence).unwrap();
        assert!(structure.is_message_table_row_at(offset));
        assert_eq!(structure.table_row_type_at(offset), None);
        let fact = CandidateFact {
            statement: evidence.to_owned(),
            segment_id: String::new(),
            evidence_quote: Some(evidence.to_owned()),
            source_byte_offset: Some(offset),
            source_locator_ambiguous: false,
            statement_byte_offset: None,
        };
        assert_eq!(
            resolve_fact_type_with_flattened_tables(document, 0, document, &fact, true, &structure),
            Err(ResolutionFailure::NoSourceType)
        );
    }
}

#[test]
fn unrelated_html_tables_do_not_declare_message_types() {
    let document = "<table><tr><td>Name</td><td>Description</td></tr><tr><td>B</td><td>ordinary</td></tr></table>";
    let structure = SourceStructure::from_document(document);
    assert_eq!(
        structure.table_row_type_at(document.find("ordinary").unwrap()),
        None
    );
}
