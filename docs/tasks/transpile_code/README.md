# Project translation (`transpile_code`) — NOT IMPLEMENTED

[Task index](../README.md) · [Project overview](../../../README.md)

## Status

**Not implemented.** The task description is a proposed contract, not an implemented translation process.

## Intended purpose

Translate a source project to a designated target language and output directory.

## Invocation

```sh
aifix -t transpile_code [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

CLI routing currently points to the generic `EndlessWorkflow` in `src/workflows/select_workflow.rs`, but there is no usable task-specific translation workflow, staged translation process or completeness validation. The task is **not implemented**; generic routing is not implementation.

## Source of truth

- [Task description](../../../task_descriptions/tasks/transpile_code.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
