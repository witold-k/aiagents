# Item documentation (`write_item_doc`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Document one important function or method with useful architectural context.

## Invocation

```sh
aifix -t write_item_doc [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

Select substantial undocumented logic; do not modify executable code.

## Source of truth

- [Task description](../../../task_descriptions/tasks/write_item_doc.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
