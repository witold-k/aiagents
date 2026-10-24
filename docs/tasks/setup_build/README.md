# Build setup (`setup_build`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Configure build directories using the build-system backend.

## Invocation

```sh
aifix -t setup_build [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

Non-LLM task handled by `Buildsystem`.

## Source of truth

- [Task description](../../../task_descriptions/tasks/setup_build.md)
- [Architecture diagrams](../../architecture_diagrams.md)
- [Build/Lint/Test workflow](../../blt_execute.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
