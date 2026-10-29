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

The workflow reads only paths changed between the selected Git revisions (added, deleted and modified paths). Renames are represented as a deletion and an addition. It does not read all repository files. Each changed file's net diff is analyzed sequentially; large diffs are split at line boundaries. File analyses are consolidated in batches, then used independently to write factual release notes and a critical technical review. The review assesses likely motivation, quality, architectural impact and risks, distinguishing verified evidence from inferences. A complete, sorted changed-files appendix is generated directly from Git rather than by the LLM. For a repository without tags, the first release is compared against Git's empty tree.

Do not infer behavior, compatibility or test success from commit messages alone.

## Source of truth

- [Task description](../../../task_descriptions/tasks/release_doc.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.

## Diagnosing information loss

Set `AIFIX_RELEASE_DOC_TRACE_DIR` to capture intermediate LLM outputs without changing the generated release document:

```sh
AIFIX_RELEASE_DOC_TRACE_DIR=.release-doc-trace aifix -t release_doc -a -f <base-ref> -o release-test.md
```

All trace filenames start with `release_doc_<repository>_`, where `<repository>` is the local repository directory name (without a trailing `.git`), sanitized for use in a filename; no remote URL is included. The directory contains release metadata (`00-*`), per-file chunk analyses (`01-*`), per-file reduction levels (`02-*`), final file analyses (`03-*`), cross-file reduction levels (`04-*`), the consolidated analysis (`05-*`), release notes (`06-*`), technical review (`07-*`) and the final assembled document (`08-*`). Compare stages to identify which facts disappeared. Trace capture is opt-in and does not change prompts or model inputs. Trace files are overwritten on repeated runs; use a fresh directory for each comparison. The trace may contain proprietary source excerpts and commit messages: keep it private and out of version control. A failed trace write is reported on stderr without stopping release generation.
