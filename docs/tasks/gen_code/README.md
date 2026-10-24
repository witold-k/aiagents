# Code generation (`gen_code`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Generate or modify Rust source before compilation.

## Invocation

```sh
aifix -t gen_code [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

Generated code requires independent build/test validation.

## Source of truth

- [Task description](../../../task_descriptions/tasks/gen_code.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
