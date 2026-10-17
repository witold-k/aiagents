// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::workflows::chunked_transform::{
    CHUNK_OVERLAP_CHARS, chunks, parse_file_records, valid_spec_path,
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
    assert!(parse_file_records("NO_MESSAGE_DEFINITIONS").unwrap().is_empty());
}

#[test]
fn mixed_empty_marker_and_records_are_rejected() {
    let response = "=== FILE: general.md ===\n# Image metadata\n=== END FILE ===\nNO_MESSAGE_DEFINITIONS";
    assert!(parse_file_records(response).is_err());
}
