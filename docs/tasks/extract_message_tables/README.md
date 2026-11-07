# Protocol fact extraction (`extract_message_tables`)

[Task index](../README.md) · [Project overview](../../../README.md)

## Purpose

Extract source-attributed candidate protocol facts from large documents, retain unresolved identities, and publish inspectable JSON. This is an **experimental extraction workflow**, not a C++ or Rust code generator and not a guarantee of a complete protocol model.

## Invocation and replay

```sh
aifix -t extract_message_tables -a document.md

# Inspect an already completed checkpoint without repeating extraction:
AIFIX_IDENTITY_REPLAY=1 aifix -t extract_message_tables -a document.md
```

Replay requires a completed extraction and coverage checkpoint. Without optional semantic verification it does not need new LLM extraction calls. The separately enabled `AIFIX_SEMANTIC_VERIFY` step **does** make LLM calls, including during replay; leave it unset for a model-call-free diagnostic run.

The task description and [Quickstart](../../../QUICKSTART.md) document invocation details.

## How the current workflow works

1. Read the document in order using overlapping source windows; tables from `simplelexer` provide supplementary structure. Tableless documents remain supported.
2. Stage `MessageCandidate` objects with source-attributed assertions. A suggested filename/name is **not** a verified message identity.
3. Preserve case-sensitive message types and unresolved candidates instead of merging by name or nearby text.
4. Check evidence quotations and statement locators against source positions. A byte-exact quote proves textual presence, **not** the interpretation of the fact.
5. Analyze parameter-like table rows and possible multi-row continuations as **diagnostics**, without automatically assigning message ownership.
6. Publish JSON specifications with provenance and review state, subject to output-directory and checkpoint rules.

The implementation also contains a document-wide source-tree shadow model, a semantic review queue and optional LLM-based review. These do not turn unverified facts into a canonical protocol schema.

## Diagnostic outputs

The audit base name is `coverage-audit.json`. The workflow can also write:

| File | What it means |
| --- | --- |
| `coverage-audit.json` | Source/fact and parameter coverage diagnostics |
| `coverage-audit.json.missing-locators.json` | Missing or uncertain statement locations |
| `coverage-audit.json.evidence-coverage.json` | Source-line evidence coverage, with overlapping windows deduplicated |
| `coverage-audit.json.semantic-review.json` | Prioritized review tasks for uncovered source material; not a verified completeness score |
| `coverage-audit.json.semantic-verification.json` | Optional LLM judgments when `AIFIX_SEMANTIC_VERIFY` is set; diagnostic only |
| `coverage-audit.json.codegen-readiness.json` | Conservative inventory of source-anchored facts and **unestablished** code-generation prerequisites |
| `coverage-audit.json.protocol-model.json` | Language-neutral diagnostic model with candidate assertions, exact source-quote status and lexical `key=value` parameter hints |

The codegen-readiness report deliberately does **not** infer field names, wire types, message ownership, cardinality, state machines, or parser/serializer behavior. The separate protocol-model report stages explicit assignment-key hints from exactly anchored source quotations; these are **lexical hypotheses**, not verified field definitions. Its status is not a quality score: “not established” does not mean “absent from the document”.

Semantic review uses a bounded, heuristic selection. A source line without extracted evidence is not necessarily a missing protocol fact; conversely, skipped lines are not proven irrelevant. Optional verification is enabled by `AIFIX_SEMANTIC_VERIFY` with `AIFIX_SEMANTIC_VERIFY_LIMIT` (default 12, capped at 200).

## Output and compatibility

The published JSON specification uses schema version 1, including `facts.json`, `manifest.json` and per-topic JSON files in `<input-stem>_messages-json` (or a diagnostic `-unverified` destination). Review status `supported` means source quotation support, **not** independent semantic correctness.

The [task description](../../../task_descriptions/tasks/extract_message_tables.md) still refers to Markdown output and `source_anchor`. The active runtime extraction prompt instead uses segmented JSON facts with `segment_id`. Do not change the task description casually: it participates in checkpoint fingerprinting.

## Current limitations and next milestone

The extraction and diagnostic JSON are not yet sufficient evidence that a C++ or Rust parser/serializer can be generated safely. In particular, canonical message identities, field definitions and constraints, encoding rules, conditional relationships, and protocol state need explicit source-backed verification.

The next milestone is a **language-independent, verified protocol model** that a separate generator can consume. Avoid specializing the general document-analysis pipeline to C++ or Rust syntax.

## References

- [Detailed chunked extraction and audit](../../chunked_transform.md)
- [Task description](../../../task_descriptions/tasks/extract_message_tables.md)
- [Architecture diagrams](../../architecture_diagrams.md)
- [Historical results](../../project_status_history.md)
