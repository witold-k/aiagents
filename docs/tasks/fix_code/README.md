# Code repair (`fix_code`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Apply an accepted repair plan to code, then return to deterministic build/lint/test validation.

## Invocation

```sh
aifix -t fix_code [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

Keep changes scoped to the selected plan; BLT validates the result.

## Source of truth

- [Task description](../../../task_descriptions/tasks/fix_code.md)
- [Architecture diagrams](../../architecture_diagrams.md)
- [Build/Lint/Test workflow](../../blt_execute.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
