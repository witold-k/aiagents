# Release notes (`release_doc`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Generate evidence-grounded Markdown release notes from Git history and diffs.

## Invocation

```sh
aifix -t release_doc [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

Do not infer behavior, compatibility or test success from commit messages alone.

## Source of truth

- [Task description](../../../task_descriptions/tasks/release_doc.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
