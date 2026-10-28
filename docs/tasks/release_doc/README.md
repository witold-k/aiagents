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

The workflow reads only paths changed between the selected Git revisions (added, deleted and modified paths). Renames are represented as a deletion and an addition. It does not read all repository files. Each changed file's net diff is analyzed sequentially; large diffs are split at line boundaries. File analyses are consolidated in batches, then used to write the final release notes. For a repository without tags, the first release is compared against Git's empty tree.\n\nDo not infer behavior, compatibility or test success from commit messages alone.

## Source of truth

- [Task description](../../../task_descriptions/tasks/release_doc.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
