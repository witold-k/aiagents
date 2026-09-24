// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::runtimetools::releasedoc::ReleaseDocContext;
use aiagents::vc::git::Git;

#[test]
fn collects_release_context_between_refs() {
    let git = Git::new(env!("CARGO_MANIFEST_DIR"));
    let context = ReleaseDocContext::between(&git, "HEAD~1", "HEAD").unwrap();

    assert_eq!(context.from.as_deref(), Some("HEAD~1"));
    assert_eq!(context.to, "HEAD");
    assert_eq!(context.commits.len(), 1);
    assert!(context.diff.as_ref().is_some_and(|diff| !diff.is_empty()));
    assert!(context
        .files
        .iter()
        .any(|file| file.path == std::path::Path::new("Cargo.toml")));
}

#[test]
fn collects_release_context_since_latest_tag() {
    let git = Git::new(env!("CARGO_MANIFEST_DIR"));
    let context = ReleaseDocContext::since_latest_tag(&git).unwrap();

    assert_eq!(context.to, "HEAD");
    assert!(!context.files.is_empty());
}


#[test]
fn formats_release_context_for_llm() {
    let git = Git::new(env!("CARGO_MANIFEST_DIR"));
    let context = ReleaseDocContext::between(&git, "HEAD~1", "HEAD").unwrap();
    let llm_context = context.to_llm_context();

    assert!(llm_context.contains("=== RELEASE RANGE ==="));
    assert!(llm_context.contains("=== COMMITS ==="));
    assert!(llm_context.contains("=== DIFF ==="));
    assert!(llm_context.contains("=== FILES AT TARGET REF ==="));
    assert!(llm_context.contains("--- FILE: Cargo.toml ---"));
}


#[test]
fn compact_context_lists_files_without_embedding_contents() {
    let git = Git::new(env!("CARGO_MANIFEST_DIR"));
    let context = ReleaseDocContext::between(&git, "HEAD~1", "HEAD").unwrap();
    let llm_context = context.to_llm_summary_context();

    assert!(llm_context.contains("=== RELEASE RANGE ==="));
    assert!(llm_context.contains("=== COMMITS ==="));
    assert!(llm_context.contains("=== DIFF ==="));
    assert!(llm_context.contains("=== AVAILABLE FILES AT TARGET REF ==="));
    assert!(llm_context.lines().any(|line| line == "Cargo.toml"));
    assert!(!llm_context.contains("--- FILE: Cargo.toml ---"));
}
