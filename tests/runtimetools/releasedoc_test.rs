// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::git_fixture::TestRepository;
use aiagents::runtimetools::releasedoc::ReleaseDocContext;

#[test]
fn collects_only_changed_files_between_refs() {
    let repository = TestRepository::new();
    let context = ReleaseDocContext::between(&repository.git(), "HEAD~1", "HEAD").unwrap();
    assert_eq!(context.commits.len(), 1);
    assert!(!context.changes.is_empty());
    assert!(context.changes.iter().all(|change| !change.diff.is_empty()));
    assert!(
        context
            .changes
            .iter()
            .all(|change| { change.diff.contains("diff --git") })
    );
}

#[test]
fn release_metadata_excludes_diff_and_repository_contents() {
    let repository = TestRepository::new();
    let context = ReleaseDocContext::since_latest_tag(&repository.git()).unwrap();
    let metadata = context.release_metadata();
    assert!(metadata.contains("=== RELEASE RANGE ==="));
    assert!(metadata.contains("=== COMMITS ==="));
    assert!(!metadata.contains("=== DIFF ==="));
    assert!(!metadata.contains("--- FILE:"));
}

#[test]
fn diff_chunking_preserves_exact_content() {
    use aiagents::workflows::release_doc::split_diff;
    let diff = "header\n@@ -1 +1 @@\n-old\n+new\n";
    let chunks = split_diff(diff, 12);
    assert!(chunks.len() > 1);
    assert_eq!(chunks.concat(), diff);
    assert_eq!(split_diff("", 12), vec![""]);
    assert_eq!(split_diff("one-long-line", 3), vec!["one-long-line"]);
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
fn release_context_includes_submodule_change_without_reading_submodule_contents() {
    use std::process::Command;
    let repository = TestRepository::new();
    let git = repository.git();
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repository.path())
        .output()
        .unwrap();
    let commit = String::from_utf8(head.stdout).unwrap();
    let gitlink = format!("160000,{},vendor/example", commit.trim());
    let staged = Command::new("git")
        .args(["update-index", "--add", "--cacheinfo", &gitlink])
        .current_dir(repository.path())
        .output()
        .unwrap();
    assert!(staged.status.success());
    let committed = Command::new("git")
        .args(["commit", "-m", "Add submodule gitlink"])
        .current_dir(repository.path())
        .output()
        .unwrap();
    assert!(committed.status.success());
    let context = ReleaseDocContext::between(&git, "HEAD~1", "HEAD").unwrap();
    assert!(context.changes.iter().any(|change| change.path
        == std::path::Path::new("vendor/example")
        && change.diff.contains("Subproject commit")));
}

#[test]
fn tracks_additions_deletions_and_renames_as_per_path_diffs() {
    use std::{fs, process::Command};
    let repository = TestRepository::new();
    let root = repository.path();
    fs::write(root.join("added.txt"), "new\n").unwrap();
    fs::rename(root.join("README.md"), root.join("renamed.md")).unwrap();
    let output = Command::new("git")
        .args(["add", "-A"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let output = Command::new("git")
        .args(["commit", "-m", "Change files"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let changes = repository
        .git()
        .changed_file_diffs("HEAD~1", "HEAD")
        .unwrap();
    for path in ["added.txt", "README.md", "renamed.md"] {
        assert!(
            changes
                .iter()
                .any(|(p, diff)| p == std::path::Path::new(path) && !diff.is_empty()),
            "{path}"
        );
    }
    assert!(
        !changes
            .iter()
            .any(|(p, _)| p == std::path::Path::new("Cargo.toml"))
    );
}

#[test]
fn first_release_without_tag_uses_empty_tree() {
    use std::process::Command;
    let repository = TestRepository::new();
    let output = Command::new("git")
        .args(["tag", "-d", "v0.1.0"])
        .current_dir(repository.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let context = ReleaseDocContext::since_latest_tag(&repository.git()).unwrap();
    assert!(context.from.is_none());
    assert!(
        context
            .changes
            .iter()
            .any(|change| change.path == std::path::Path::new("README.md"))
    );
}

#[test]
fn assembled_release_document_contains_independent_review_and_all_changed_paths() {
    use aiagents::runtimetools::releasedoc::ReleaseFileChange;
    use aiagents::workflows::release_doc::assemble_release_document;
    let changes = vec![
        ReleaseFileChange {
            path: "src/z.rs".into(),
            diff: "diff".into(),
        },
        ReleaseFileChange {
            path: "src/a.rs".into(),
            diff: "diff".into(),
        },
        ReleaseFileChange {
            path: "src/z.rs".into(),
            diff: "diff".into(),
        },
    ];
    let result = assemble_release_document(
        "# Release Notes\n\n## Changes\n- New behavior.\n",
        "### Quality\nThe design is simpler, but error handling needs review.",
        &changes,
    );
    assert!(result.starts_with("# Release Notes\n"));
    assert!(result.contains("## Technical Review\n\n### Quality\n"));
    assert!(result.contains("## Changed Files\n\n- `src/a.rs`\n- `src/z.rs`\n"));
    assert_eq!(result.matches("- `src/z.rs`").count(), 1);
}

#[test]
fn assembled_release_document_handles_empty_change_list() {
    use aiagents::workflows::release_doc::assemble_release_document;
    let result = assemble_release_document("# Release Notes", "No issues identified.", &[]);
    assert!(result.contains("## Technical Review\n\nNo issues identified."));
    assert!(result.ends_with("## Changed Files\n\nNo changed files.\n"));
}

#[test]
fn assembled_release_document_removes_repeated_heading_and_document_fences() {
    use aiagents::workflows::release_doc::assemble_release_document;
    let result = assemble_release_document(
        "```markdown\n# Release Notes\n\n## Changes\n- One change.\n```",
        "```markdown\n## Technical Review\n\n### Quality\nConcrete observation.\n```",
        &[],
    );
    assert!(result.starts_with("# Release Notes\n"));
    assert_eq!(result.matches("## Technical Review").count(), 1);
    assert!(result.contains("## Technical Review\n\n### Quality\nConcrete observation."));
    assert!(!result.contains("```"));
}

#[test]
fn assembled_release_document_preserves_review_without_repeated_heading() {
    use aiagents::workflows::release_doc::assemble_release_document;
    let result = assemble_release_document(
        "# Release Notes",
        "### Quality\n## Technical Review is mentioned in prose, not as heading.",
        &[],
    );
    assert!(result.contains("### Quality\n## Technical Review is mentioned"));
}

#[test]
fn review_input_contains_original_diffs_without_llm_summaries() {
    use aiagents::runtimetools::releasedoc::ReleaseFileChange;
    use aiagents::workflows::release_doc::build_review_input;
    let changes = vec![ReleaseFileChange {
        path: "src/a.rs".into(),
        diff: "diff --git a/src/a.rs b/src/a.rs\n@@ -1 +1 @@\n-old\n+new\n".into(),
    }];
    let input = build_review_input("COMMITS", &changes);
    assert!(input.contains("ORIGINAL GIT DIFFS"));
    assert!(input.contains("=== FILE: src/a.rs ==="));
    assert!(input.contains("+new"));
    assert!(!input.contains("LLM-GENERATED"));
}

#[test]
fn review_input_omits_entire_oversized_diff_with_explicit_path() {
    use aiagents::runtimetools::releasedoc::ReleaseFileChange;
    use aiagents::workflows::release_doc::build_review_input;
    let changes = vec![
        ReleaseFileChange {
            path: "src/huge.rs".into(),
            diff: "x".repeat(64_001),
        },
        ReleaseFileChange {
            path: "tests/small.rs".into(),
            diff: "+small".into(),
        },
    ];
    let input = build_review_input("COMMITS", &changes);
    assert!(input.contains("=== FILE: tests/small.rs ==="));
    assert!(input.contains("+small"));
    assert!(input.contains("OMITTED FILE DIFFS (NOT REVIEWED)"));
    assert!(input.contains("- src/huge.rs"));
    assert!(!input.contains(&"x".repeat(100)));
}

#[test]
fn validation_accepts_only_explicit_ok() {
    use aiagents::workflows::release_doc::review_validation_is_ok;
    assert!(review_validation_is_ok("OK\n"));
    assert!(!review_validation_is_ok("REVISE\n- missing error path"));
    assert!(!review_validation_is_ok("OK, but one issue remains"));
    assert!(!review_validation_is_ok(""));
}

#[test]
fn validation_and_revision_keep_original_evidence_and_feedback() {
    use aiagents::workflows::release_doc::{
        build_review_revision_input, build_review_validation_input,
    };
    let original = "=== ORIGINAL GIT DIFFS ===\n+return WorkflowResult::Error";
    let draft = "### Trace\n- **Consequence:** Writes never fail.";
    let validation = build_review_validation_input(original, draft);
    assert!(validation.contains(original));
    assert!(validation.contains(draft));
    let revised = build_review_revision_input(&validation, "REVISE\n- distinguish create_dir_all");
    assert!(revised.contains(original));
    assert!(revised.contains(draft));
    assert!(revised.contains("distinguish create_dir_all"));
}
