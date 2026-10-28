# Code generation (`gen_code`) — NOT IMPLEMENTED

[Task index](../README.md) · [Project overview](../../../README.md)

## Status

**Not implemented.** The existing task description expresses intended behavior, not a usable implemented task.

## Intended purpose

Generate or modify Rust source before compilation.

## Invocation

```sh
aifix -t gen_code [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

CLI routing currently points to the generic `EndlessWorkflow`, but there is no task-specific generation workflow or coherent build/test validation process. The generic route must not be counted as an implementation.

## Source of truth

- [Task description](../../../task_descriptions/tasks/gen_code.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
