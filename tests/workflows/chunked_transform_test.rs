// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::workflows::chunked_transform::{
    CHUNK_OVERLAP_CHARS, append_reviewed_fragment, apply_audit_edit, chunks, coverage_message_path,
    mask_embedded_data_uris, mask_opaque_lines, merge_preserves_existing, parse_audit_decision,
    parse_file_records, valid_spec_path,
};

#[test]
fn short_document_is_unchanged() {
    let source = "a\nb\n\nc\nd\n\ne";
    let parts = chunks(source, 100);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].text, source);
    assert_eq!(parts[0].start_line, 1);
    assert_eq!(parts[0].end_line, 7);
}

#[test]
fn adjacent_chunks_overlap_exactly() {
    let source = "x".repeat(3000);
    let parts = chunks(&source, 1200);
    assert!(parts.len() > 2);
    for pair in parts.windows(2) {
        assert_eq!(pair[0].text.chars().count(), 1200);
        let tail: String = pair[0].text.chars().skip(200).collect();
        assert_eq!(
            tail,
            pair[1]
                .text
                .chars()
                .take(CHUNK_OVERLAP_CHARS)
                .collect::<String>()
        );
    }
    assert!(parts.iter().all(|part| part.text.chars().count() <= 1200));
}

#[test]
fn unicode_and_newlines_preserve_source_with_overlap() {
    let source = "😀é\n".repeat(2000);
    let parts = chunks(&source, 1100);
    let mut recovered = parts[0].text.clone();
    for part in parts.iter().skip(1) {
        recovered.extend(part.text.chars().skip(CHUNK_OVERLAP_CHARS));
    }
    assert_eq!(recovered, source);
    assert!(parts.iter().all(|part| part.text.chars().count() <= 1100));
}

#[test]
fn embedded_image_data_remains_bounded() {
    let source = format!(
        "![figure](data:image/png;base64,{})\nNext",
        "A".repeat(50_000)
    );
    let parts = chunks(&source, 12_000);
    assert!(parts.len() >= 5);
    assert!(parts.iter().all(|part| part.text.chars().count() <= 12_000));
}

#[test]
fn empty_document_has_no_chunks() {
    assert!(chunks("", 100).is_empty());
    assert!(chunks("a", 0).is_empty());
}

#[test]
fn one_character_windows_make_progress() {
    let parts = chunks("abc", 1);
    assert_eq!(
        parts.iter().map(|p| p.text.as_str()).collect::<String>(),
        "abc"
    );
}
#[test]
fn parses_multiple_specification_files() {
    let response = "=== FILE: general.md ===\n# General\n=== END FILE ===\n=== FILE: messages/a-1.md ===\n# A\n=== END FILE ===";
    let records = parse_file_records(response).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[1].0, "messages/a-1.md");
}

#[test]
fn rejects_unsafe_or_malformed_model_paths() {
    assert!(!valid_spec_path("messages/../escape.md"));
    assert!(!valid_spec_path("README.md"));
    assert!(!valid_spec_path("messages/A.md"));
    assert!(parse_file_records("=== FILE: ../oops.md ===\nfoo\n=== END FILE ===").is_err());
    assert!(parse_file_records("garbage").is_err());
    assert!(parse_file_records("=== FILE: general.md ===\nfoo").is_err());
}

#[test]
fn irrelevant_chunk_uses_empty_marker() {
    assert!(
        parse_file_records("NO_MESSAGE_DEFINITIONS")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn trailing_empty_marker_does_not_discard_complete_records() {
    let response =
        "=== FILE: general.md ===\n# Image metadata\n=== END FILE ===\nNO_MESSAGE_DEFINITIONS";
    assert_eq!(parse_file_records(response).unwrap().len(), 1);
}

#[test]
fn accepts_markdown_wrapped_records_and_commentary() {
    let response = "Here are the corrected file records:\n\n=== FILE: messages/p.md ===\n```markdown\n# Polarization\n\nField: H or V\n```\n=== END FILE ===\n=== NO_MESSAGE_DEFINITIONS ===";
    let records = parse_file_records(response).unwrap();
    assert_eq!(
        records,
        vec![(
            "messages/p.md".into(),
            "# Polarization\n\nField: H or V".into()
        )]
    );
}

#[test]
fn accepts_decorated_empty_marker() {
    assert!(
        parse_file_records("=== NO_MESSAGE_DEFINITIONS ===")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn rejects_truncated_or_nested_records_after_normalization() {
    assert!(parse_file_records("=== FILE: messages/p.md ===\n```markdown\n# P").is_err());
    assert!(
        parse_file_records(
            "=== FILE: messages/p.md ===\n# P\n=== FILE: messages/q.md ===\n# Q\n=== END FILE ==="
        )
        .is_err()
    );
}

#[test]
fn audit_empty_marker_is_unchanged_but_other_invalid_decisions_fail() {
    assert_eq!(parse_audit_decision("UNCHANGED"), Ok(false));
    assert_eq!(parse_audit_decision("REVISE"), Ok(true));
    assert_eq!(parse_audit_decision("NO_MESSAGE_DEFINITIONS"), Ok(false));
    assert_eq!(
        parse_audit_decision("=== NO_MESSAGE_DEFINITIONS ==="),
        Ok(false)
    );
    assert!(parse_audit_decision("Maybe unchanged").is_err());
}

#[test]
fn masks_embedded_image_data_without_changing_chunk_boundaries() {
    let original = format!(
        "Header\n![plot](data:image/png;base64,{})\nMessage M\n",
        "A".repeat(20_000)
    );
    let masked = mask_embedded_data_uris(&original);
    assert_eq!(original.len(), masked.len());
    assert_eq!(original.lines().count(), masked.lines().count());
    assert!(masked.contains("Message M"));
    assert!(!masked.contains(&"A".repeat(128)));
    let raw_chunks = chunks(&original, 4_000);
    let masked_chunks = chunks(&masked, 4_000);
    assert_eq!(raw_chunks.len(), masked_chunks.len());
    for (raw, clean) in raw_chunks.iter().zip(&masked_chunks) {
        assert_eq!(
            (raw.start_line, raw.end_line),
            (clean.start_line, clean.end_line)
        );
    }
}

#[test]
fn preserves_base64_like_protocol_fields_and_short_data_uris() {
    let source = "payload=ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\ndata:image/png;base64,AAAA";
    assert_eq!(mask_embedded_data_uris(source), source);
}

#[test]
fn masks_long_encoded_runs_without_changing_source_positions() {
    let source = format!(
        "Protocol r command\n![image](opaque:{})\nProtocol s command\n",
        "A".repeat(8000)
    );
    let cleaned = mask_opaque_lines(&source);
    assert_eq!(source.len(), cleaned.len());
    assert_eq!(source.lines().count(), cleaned.lines().count());
    assert!(cleaned.contains("Protocol r command"));
    assert!(cleaned.contains("Protocol s command"));
    assert!(!cleaned.contains(&"A".repeat(512)));
}

#[test]
fn preserves_short_protocol_tokens_and_unicode() {
    let source = "Befehl: äöü r 1.25\nfield=abc123\n";
    assert_eq!(mask_opaque_lines(source), source);
}

#[test]
fn audit_edit_requires_unique_old_text_and_source_quote() {
    let existing = "Rule: old value.\\nOther rule unchanged.";
    let evidence = "The correct rule is new value.";
    let valid = r#"{"evidence_quote":"The correct rule is new value.","old_text":"old value","new_text":"new value"}"#;
    assert_eq!(
        apply_audit_edit(existing, evidence, valid).unwrap(),
        "Rule: new value.\\nOther rule unchanged."
    );
    let unsupported =
        r#"{"evidence_quote":"not in the source","old_text":"old value","new_text":"new value"}"#;
    assert!(apply_audit_edit(existing, evidence, unsupported).is_err());
    let repeated = r#"{"evidence_quote":"The correct rule is new value.","old_text":"rule","new_text":"field"}"#;
    assert!(apply_audit_edit("rule and rule", evidence, repeated).is_err());
}

#[test]
fn audit_edit_accepts_complete_json_fence_but_rejects_extra_prose() {
    let existing = "Rule: old value.";
    let evidence = "The correct rule is new value.";
    let json = r#"{"evidence_quote":"The correct rule is new value.","old_text":"old value","new_text":"new value"}"#;
    let fenced = format!("```json\n{json}\n```");
    assert_eq!(
        apply_audit_edit(existing, evidence, &fenced).unwrap(),
        "Rule: new value."
    );
    let with_prose = format!("{fenced}\nExtra commentary");
    assert!(apply_audit_edit(existing, evidence, &with_prose).is_err());
    assert!(apply_audit_edit(existing, evidence, "```json\n{}").is_err());
}

#[test]
fn audit_edit_rejects_json_block_embedded_in_commentary() {
    let existing = "Rule: old value.";
    let evidence = "The correct rule is new value.";
    let json = r#"{"evidence_quote":"The correct rule is new value.","old_text":"old value","new_text":"new value"}"#;
    let embedded = format!("Here is my correction:\n```json\n{json}\n```");
    assert!(apply_audit_edit(existing, evidence, &embedded).is_err());
    let wrong_language = format!("```markdown\n{json}\n```");
    assert!(apply_audit_edit(existing, evidence, &wrong_language).is_err());
}

#[test]
fn merge_must_preserve_existing_specification_verbatim() {
    let original = "# Message A\n\nField: uint16";
    assert!(merge_preserves_existing(
        original,
        "# Message A\n\nField: uint16\n\nAdditional field: uint8"
    ));
    assert!(!merge_preserves_existing(
        original,
        "# Message A\n\nField: uint32"
    ));
    assert!(!merge_preserves_existing(original, ""));
}

#[test]
fn reviewed_fragments_accumulate_without_rewriting_existing_text() {
    let first = "# Shared rules\n\nA rule.";
    let next = "# Shared rules\n\nB rule.";
    let combined = append_reviewed_fragment(first, next);
    assert!(combined.starts_with(first));
    assert!(combined.contains(next));
    assert_eq!(append_reviewed_fragment(&combined, next), combined);
    assert_eq!(append_reviewed_fragment(&combined, "  "), combined);
}

#[test]
fn coverage_accepts_only_message_destinations() {
    assert!(coverage_message_path("messages/s.md"));
    assert!(!coverage_message_path("general.md"));
    assert!(!coverage_message_path("infrastructure.md"));
    assert!(!coverage_message_path("messages/NO_MESSAGE_DEFINITIONS.md"));
    assert!(!coverage_message_path("../messages/s.md"));
}
