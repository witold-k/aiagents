# Protocol fact extraction (`extract_message_tables`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Extract source-grounded protocol facts from large documents into JSON diagnostic specifications.

## Invocation

```sh
aifix -t extract_message_tables [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

Experimental P4: checkpointed candidates, case-sensitive identity projection and read-only replay. Quote support is not semantic verification.

## Source of truth

- [Task description](../../../task_descriptions/tasks/extract_message_tables.md)
- [Architecture diagrams](../../architecture_diagrams.md)
- [Chunked extraction and audit](../../chunked_transform.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
