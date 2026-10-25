# Test generation (`write_test_code`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Generate or extend automated tests for selected source files.

## Invocation

```sh
aifix -t write_test_code [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

Preserve existing tests and production code; verify generated tests with the build system.

## Source of truth

- [Task description](../../../task_descriptions/tasks/write_test_code.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
