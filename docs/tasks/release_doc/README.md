# Release notes (`release_doc`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Generate evidence-grounded Markdown release notes from Git history and per-file net diffs.

## Invocation

```sh
# Print release notes to stdout (default):
aifix -t release_doc

# Write the default filename RELEASE_NOTES.md:
aifix -t release_doc -a -o

# Write a specific filename:
aifix -t release_doc -a -o docs/releases/v1.0.md

# Select a Git range and a destination:
aifix -t release_doc -a -f v0.1.0 -l HEAD -o notes.md
```

Options specific to `release_doc` follow `-a` / `--args`. The output path is relative to the project directory unless absolute. A parent directory must already exist.

## Behavior and boundaries

Without `-o`, the generated Markdown is printed to stdout; no release notes file is written by the workflow. With `-o` but without a filename the result is written to `RELEASE_NOTES.md`. Supplying a filename overrides that default.

The workflow reads only paths changed between the selected Git revisions (added, deleted and modified paths). Renames are represented as a deletion and an addition. It does not read all repository files. Each changed file's net diff is analyzed sequentially; large diffs are split at line boundaries. File analyses are consolidated in batches, then used independently to write factual release notes and a critical technical review. The review evaluates changed code for supported technical findings and risks, distinguishing visible mechanisms from inferences. A separate validation LLM call checks the review against the original Git diffs. Only an exact `OK` response accepts it unchanged; other successful validation responses trigger one revision attempt. Validation is model-generated feedback, not independent proof of correctness. A complete, sorted changed-files appendix is generated directly from Git rather than by the LLM. For a repository without tags, the first release is compared against Git's empty tree.

Do not infer behavior, compatibility or test success from commit messages alone.

## Review failures and incomplete evidence

The technical review is best-effort. Review, validation, or revision LLM calls can fail (for example because the model's input or output token limits are exceeded, a large or binary diff is involved, or the provider returns an unrelated error). Such failures no longer discard already generated release notes. The final document retains the Git-derived changed-files list. A failed review produces an unavailable-review explanation; a failed validation retains the draft review with a **NOT VALIDATED** marker; a failed revision retains the draft review and validation feedback with a **VALIDATION FAILED; REVISION UNAVAILABLE** marker. Failures of review, validation, or revision are reported on stderr. A successful revision is marked **REVISED, NOT REVALIDATED**; there is no second validation pass.

The review input uses a **shared** 64,000-byte budget for complete per-file diffs, in addition to release metadata and prompts. Files are considered in priority groups: `src/` and `build.rs` first, `tests/` second, and all other paths last. Within a group, the original change-list order is preserved; this is a selection priority, not a guarantee of complete coverage. Diffs that do not fit are explicitly listed as omitted and are **not reviewed**. The initial per-file analysis splits at line boundaries; exceptionally long lines can still exceed its nominal chunk size. Consequently, neither the presence of a technical review nor a successful validation establishes completeness, especially for binary files or large changes. Check the original Git diff when accuracy matters.

## Source of truth

- [Task description](../../../task_descriptions/tasks/release_doc.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.

## Diagnosing information loss

Set `AIFIX_RELEASE_DOC_TRACE_DIR` to capture intermediate LLM outputs without changing the generated release document (replace `BASE_REF` with a Git revision):

```sh
AIFIX_RELEASE_DOC_TRACE_DIR=.release-doc-trace aifix -t release_doc -a -f BASE_REF -o release-test.md
```

All trace filenames start with `release_doc_<repository>_`, where `<repository>` is the local repository directory name (without a trailing `.git`), sanitized for use in a filename; no remote URL is included. The directory contains release metadata (`00-*`), per-file chunk analyses (`01-*`), per-file reduction levels (`02-*`), final file analyses (`03-*`), cross-file reduction levels (`04-*`), the consolidated analysis (`05-*`), release notes (`06-release-notes.md`), original review input (`07-review-input.txt`), draft technical review (`07-technical-review.md`), validation feedback (`08-review-validation.md`), an optional revised review (`09-revised-review.md`), and the final assembled document (`10-final-document.md`). Validation and revision traces are present only when their respective stages complete successfully. Compare stages to identify which facts disappeared. Trace capture is opt-in and does not change prompts or model inputs. Trace files are overwritten on repeated runs; use a fresh directory for each comparison. The trace may contain proprietary source excerpts and commit messages: keep it private and out of version control. A failed trace write is reported on stderr without stopping release generation.
