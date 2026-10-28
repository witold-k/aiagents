# Build (`build`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Run configured build-system commands deterministically.

## Invocation

```sh
aifix -t build [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

`BuildWorkflow::build()` executes the configured build command. If it fails, `Workflow::execute_build_llm()` invokes the LLM with the diagnostic. Therefore this task is **not strictly non-LLM on failure**; it does not run the full build/lint/test sequence.

## Source of truth

- [Task description](../../../task_descriptions/tasks/build.md)
- [Architecture diagrams](../../architecture_diagrams.md)
- [Build/Lint/Test workflow](../../blt_execute.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
