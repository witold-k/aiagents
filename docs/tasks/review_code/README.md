# Code review (`review_code`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Analyze source and write non-destructive per-file review reports.

## Invocation

```sh
aifix -t review_code [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

Writes companion `<filename>.review.md` files; source code must not be modified.

## Source of truth

- [Task description](../../../task_descriptions/tasks/review_code.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
