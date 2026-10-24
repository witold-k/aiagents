# Module documentation (`write_module_doc`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Prepend a module-level overview explaining responsibilities, components and integration.

## Invocation

```sh
aifix -t write_module_doc [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

Keep all existing code and imports unchanged.

## Source of truth

- [Task description](../../../task_descriptions/tasks/write_module_doc.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
