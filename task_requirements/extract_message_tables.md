# Task Requirements: extract_message_tables

## Purpose

Implement the `extract_message_tables` task in `aiagents`. Transform a
potentially very long Markdown (`.md`) or plain-text (`.txt`) protocol
description into one formal, consolidated Markdown specification of all
messages described in the source.

This file is the **implementation brief / generation instructions** for the
task and workflow, not an input protocol and not the resulting specification.
Keep the actual LLM instructions in
`task_descriptions/tasks/extract_message_tables.md` and
`task_descriptions/workflow_steps/`.

## Input and output

- Input: exactly one existing `.md` or `.txt` document, selected explicitly
  by path; never modify it.
- Output: `<input_stem>_messages.md` next to the input file.
- Produce **only Markdown**. No executable code, Rust/C++ files, generated
  implementations, build steps, or project changes.
- The output must be usable as an unambiguous specification for a *later*,
  separate code-generation task.

## Required extracted information

Extract all protocol messages, whether described by tables, prose, examples,
or a combination. Include, where present:

1. Message names, IDs/opcodes, direction, purpose, framing, and relationships.
2. Complete field layouts: order, byte/bit offsets, sizes, types, signedness,
   endian/bit order, alignment, padding, reserved bits, defaults, and units.
3. Enums, flags, allowed ranges, special values, optional and conditional
   fields, unions, variants, repetition, arrays, and variable-length fields.
4. Message size formulas, minimum and maximum lengths, delimiters, escaping,
   length fields, checksums/CRC, and parsing/serialization rules.
5. Version-specific differences, state-dependent behavior, constraints,
   validation rules, errors, and dependencies on shared definitions.
6. Source references (at least original line ranges) for every message and
   relevant rule; flag ambiguous, missing, or contradictory facts.

Do not invent missing technical facts. Distinguish verified requirements from
assumptions, recommendations, and open questions.

## Memory and helper abstractions

For each message, analyze whether its representation and parsing require
dynamic allocation. **Assume neither that dynamic allocation is needed nor
that it is impossible.** Prefer fixed-size or bounded buffers when source
bounds allow them. State:

- Fixed versus variable length, maximum size if derivable, and any unknown
  upper bound.
- Whether static/stack/bounded storage is sufficient, with reasoning.
- Cases where dynamic allocation may actually be needed and why.
- Necessary *formal* helper abstractions (e.g. bounded byte reader, string
  parser, endian decoder, checksum validator): responsibility, input/output
  contract, bounds, error conditions, and dependencies.

These are specifications of interfaces and behavior, **not code**.

## Architecture: reusable chunked_transform workflow

Implement `ChunkedTransformWorkflow` as a generic orchestration mechanism.
`extract_message_tables` supplies domain-specific prompts and validation
requirements; the workflow owns chunking, orchestration, and output assembly.
Keep it independent of any build system or programming language.

### Stage 1 — Source-aware chunking

- Process documents larger than the LLM context window with bounded input
  chunks; never send the entire source to one model call.
- Prefer Markdown heading/paragraph boundaries and **do not split tables
  between header and rows** when avoidable.
- Handle plain-text tables and long tables explicitly. When an indivisible
  structure exceeds the chunk budget, split it safely with repeated headers,
  continuation markers, and bounded overlap/context.
- Maintain exact source positions and cover every input line. Ensure overlap
  does not cause duplicated output.
- Retain globally applicable definitions (framing, endian, shared field types,
  common headers) and references between distant source sections, without
  growing the prompt without bound.

### Stage 2 — Extract

- Run the extraction prompt for every chunk, preserving original line ranges.
- Extract complete facts and explicit continuations, not summaries that drop
  fields or constraints.
- Preserve source citations, unresolved references, and partial definitions.
- Represent empty chunks explicitly so they cannot be mistaken for a failure.

### Stage 3 — Merge and normalize

- Merge message fragments spread across chunks into **one logical definition**
  per message/version, rather than simply concatenating chunk outputs.
- Deduplicate overlapping source facts, normalize Markdown headings/tables,
  and resolve cross-references to shared definitions.
- Keep conflicting source statements visible; do not silently choose one.
- The merge itself must be hierarchical or chunked when intermediate results
  exceed the model context window.

### Stage 4 — Independent validation and repair

- Verify extracted fields, offsets, sizes, constraints, and omissions against
  the **corresponding original source excerpts**, not just model output.
- Revisit all source ranges contributing to a merged message; check that
  cross-chunk continuations were incorporated correctly.
- Correct discrepancies and rerun bounded checks as needed, with a finite
  retry limit and explicit failure if validation cannot be completed.
- Do not load the entire source or complete output into one validation call.
- Report unresolved ambiguities honestly; do not label unverified output as
  fully validated.

### Stage 5 — Write result

- Write a single `<input_stem>_messages.md` only after successful assembly
  and required checks.
- Use a consistent, readable specification structure: overview, shared
  definitions, message index, one section per message, field tables, size and
  allocation analysis, parsing/validation rules, helper abstraction
  specifications, source references, and unresolved questions.
- Do not overwrite the input. Avoid partial/corrupt output on failures.

## Acceptance criteria / tests

1. Short Markdown and plain-text documents work.
2. Long documents spanning many chunks work without unbounded model context.
3. Tables crossing chunk boundaries retain their headers and every row.
4. A message described across distant sections is merged once, correctly.
5. Shared definitions (e.g. endian and common headers) reach relevant messages.
6. Independent verification detects missing or altered fields.
7. Overlap does not duplicate messages or fields.
8. Output is a single valid Markdown file with traceable source line ranges.
9. Static/bounded memory is preferred when justified; unknown bounds remain
   explicit; helper classes are specified but not implemented.
10. No source code is generated and the input file is unchanged.
11. Build, tests, Clippy, and formatting checks pass.
12. Errors in reading, parsing, model responses, merging, validation, or
    writing are reported rather than silently producing a success result.

## Implementation note

The initial feature-branch implementation is a **first iteration**: it has
bounded per-chunk extraction and review, but its current line/paragraph
chunker is not yet fully table-aware, and it does not perform a true
cross-chunk semantic merge or global validation. These are requirements
still to implement before treating the workflow as complete.
