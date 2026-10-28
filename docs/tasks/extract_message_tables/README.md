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

Experimental P4: checkpointed candidates, case-sensitive identity projection and read-only replay. Quote support is not semantic verification. The 28 October OpenAMIP checkpoint diagnostic retained 243 candidates and 934 fact occurrences; 187 candidates remain unresolved. Of 655 exact statement locations, 8 statement/evidence pairs occupy different table rows and 1 different sections. These are diagnostic observations, not inferred identities. The provenance audit checks statement locations independently of evidence-quote anchors; neither a matching quote nor a proposed source-tree owner establishes semantic correctness.

## Contract discrepancy

The task description currently mentions Markdown specifications and `source_anchor`, while the runtime extraction prompt in `src/workflows/chunked_transform.rs` requests segmented JSON facts with `segment_id` and stages JSON publication. For current behavior, inspect the runtime code. Updating the task description is a separate compatibility-sensitive change because it participates in checkpoint fingerprinting.

## Source of truth

- [Task description](../../../task_descriptions/tasks/extract_message_tables.md)
- [Architecture diagrams](../../architecture_diagrams.md)
- [Chunked extraction and audit](../../chunked_transform.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
