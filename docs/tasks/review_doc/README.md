# Document review (`review_doc`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Review documents, specifications or transcripts for accuracy, consistency and omissions.

## Invocation

```sh
aifix -t review_doc [task-specific arguments]
```

The example shows task selection only. Check the task definition and [Quickstart](../../../QUICKSTART.md) for applicable arguments and configured access.

## Behavior and boundaries

This task uses the shared `DocWorkflow`, not a dedicated document-review workflow. Its prompt requests review output; the runtime does not independently enforce a complete review.


Writes companion `<filename>.review.md` files; original documents remain unchanged.

## Source of truth

- [Task description](../../../task_descriptions/tasks/review_doc.md)
- [Architecture diagrams](../../architecture_diagrams.md)

## Validation

Task results are not automatically proof of correctness. Validate modified source with appropriate build, lint and test commands, and verify generated documentation or extracted claims against their sources.
