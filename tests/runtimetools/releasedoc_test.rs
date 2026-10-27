// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::runtimetools::releasedoc::ReleaseDocContext;

use crate::git_fixture::TestRepository;

#[test]
fn collects_release_context_between_refs() {
    let repository = TestRepository::new();
    let git = repository.git();
    let context = ReleaseDocContext::between(&git, "HEAD~1", "HEAD").unwrap();

    assert_eq!(context.from.as_deref(), Some("HEAD~1"));
    assert_eq!(context.to, "HEAD");
    assert_eq!(context.commits.len(), 1);
    assert!(context.diff.as_ref().is_some_and(|diff| !diff.is_empty()));
    assert!(
        context
            .files
            .iter()
            .any(|file| file.path == std::path::Path::new("Cargo.toml"))
    );
}

#[test]
fn collects_release_context_since_latest_tag() {
    let repository = TestRepository::new();
    let git = repository.git();
    let context = ReleaseDocContext::since_latest_tag(&git).unwrap();

    assert_eq!(context.from.as_deref(), Some("v0.1.0"));
    assert_eq!(context.to, "HEAD");
    assert_eq!(context.commits.len(), 1);
    assert!(
        context
            .files
            .iter()
            .any(|file| file.path == std::path::Path::new("Cargo.toml"))
    );
}

#[test]
fn formats_release_context_for_llm() {
    let repository = TestRepository::new();
    let git = repository.git();
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
    let repository = TestRepository::new();
    let git = repository.git();
    let context = ReleaseDocContext::between(&git, "HEAD~1", "HEAD").unwrap();
    let llm_context = context.to_llm_summary_context();

    assert!(llm_context.contains("=== RELEASE RANGE ==="));
    assert!(llm_context.contains("=== COMMITS ==="));
    assert!(llm_context.contains("=== DIFF ==="));
    assert!(llm_context.contains("=== AVAILABLE FILES AT TARGET REF ==="));
    assert!(llm_context.lines().any(|line| line == "Cargo.toml"));
    assert!(!llm_context.contains("--- FILE: Cargo.toml ---"));
}

use aiagents::workflows::release_doc::{ReleaseDocOptions, ReleaseDocOutput, write_release_notes};
use std::path::PathBuf;

fn release_options(args: &[&str]) -> ReleaseDocOptions {
    ReleaseDocOptions::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
}

#[test]
fn release_notes_default_to_stdout() {
    let options = release_options(&[]);
    assert_eq!(options.output, ReleaseDocOutput::Stdout);

    let mut stdout = Vec::new();
    write_release_notes(
        &options.output,
        std::path::Path::new("."),
        "# Notes\n",
        &mut stdout,
    )
    .unwrap();
    assert_eq!(stdout, b"# Notes\n");
}

#[test]
fn release_notes_output_flag_uses_default_file() {
    assert_eq!(
        release_options(&["-o"]).output,
        ReleaseDocOutput::File(PathBuf::from("RELEASE_NOTES.md"))
    );
    assert_eq!(
        release_options(&["--output"]).output,
        ReleaseDocOutput::File(PathBuf::from("RELEASE_NOTES.md"))
    );
}

#[test]
fn release_notes_support_custom_file_and_git_range_in_any_order() {
    let options = release_options(&["-f", "v0.1.0", "-o", "notes.md", "-l", "HEAD"]);
    assert_eq!(options.first.as_deref(), Some("v0.1.0"));
    assert_eq!(options.last, "HEAD");
    assert_eq!(
        options.output,
        ReleaseDocOutput::File(PathBuf::from("notes.md"))
    );
    let reverse = release_options(&["--output", "docs/notes.md", "--first", "v0.1.0"]);
    assert_eq!(
        reverse.output,
        ReleaseDocOutput::File(PathBuf::from("docs/notes.md"))
    );
    assert_eq!(reverse.first.as_deref(), Some("v0.1.0"));
}

#[test]
fn release_notes_output_flag_does_not_consume_next_option() {
    let options = release_options(&["-o", "-f", "v0.1.0"]);
    assert_eq!(options.first.as_deref(), Some("v0.1.0"));
    assert_eq!(
        options.output,
        ReleaseDocOutput::File(PathBuf::from("RELEASE_NOTES.md"))
    );
}

#[test]
fn release_notes_reject_invalid_arguments() {
    for args in [
        vec!["-f"],
        vec!["--last"],
        vec!["-f", "-o"],
        vec!["--output", ""],
        vec!["--unknown"],
    ] {
        let owned = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(ReleaseDocOptions::parse(&owned).is_err(), "{args:?}");
    }
}

#[test]
fn release_notes_file_output_writes_exact_content() {
    let tmp = std::env::temp_dir().join(format!(
        "aiagents-release-doc-test-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let mut stdout = Vec::new();
    write_release_notes(
        &ReleaseDocOutput::File(PathBuf::from("custom.md")),
        &tmp,
        "# Changelog\n",
        &mut stdout,
    )
    .unwrap();
    assert!(stdout.is_empty());
    assert_eq!(
        std::fs::read_to_string(&tmp.join("custom.md")).unwrap(),
        "# Changelog\n"
    );
}

#[test]
fn release_context_skips_submodule_gitlinks_without_checkout() {
    use std::process::Command;

    let repository = TestRepository::new();
    let git = repository.git();
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repository.path())
        .output()
        .unwrap();
    assert!(head.status.success());
    let commit = String::from_utf8(head.stdout).unwrap();
    let gitlink = format!("160000,{},vendor/example", commit.trim());
    let staged = Command::new("git")
        .args(["update-index", "--add", "--cacheinfo", &gitlink])
        .current_dir(repository.path())
        .output()
        .unwrap();
    assert!(
        staged.status.success(),
        "{}",
        String::from_utf8_lossy(&staged.stderr)
    );

    let committed = Command::new("git")
        .args(["commit", "-m", "Add submodule gitlink"])
        .current_dir(repository.path())
        .output()
        .unwrap();
    assert!(
        committed.status.success(),
        "{}",
        String::from_utf8_lossy(&committed.stderr)
    );

    let files = git.files("HEAD").unwrap();
    assert!(
        files
            .iter()
            .any(|path| path == std::path::Path::new("Cargo.toml"))
    );
    assert!(
        !files
            .iter()
            .any(|path| path == std::path::Path::new("vendor/example"))
    );

    let context = ReleaseDocContext::between(&git, "HEAD~1", "HEAD").unwrap();
    assert!(
        context
            .files
            .iter()
            .all(|file| file.path != std::path::Path::new("vendor/example"))
    );
    assert!(
        context
            .diff
            .as_ref()
            .is_some_and(|diff| diff.contains("vendor/example"))
    );
}
