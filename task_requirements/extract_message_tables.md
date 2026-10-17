# Task Requirements: extract_message_tables

## Purpose

Implement the `extract_message_tables` task in `aiagents`. Transform a
potentially very long Markdown (`.md`) or plain-text (`.txt`) protocol
description into a directory of formal Markdown specifications, with one file per
logical message/version and separate shared protocol and helper contracts.

This file is the **implementation brief / generation instructions** for the
task and workflow, not an input protocol and not the resulting specification.
Keep the actual LLM instructions in
`task_descriptions/tasks/extract_message_tables.md` and
`task_descriptions/workflow_steps/`.

## Input and output

- Input: exactly one existing `.md` or `.txt` document, selected explicitly
  by path; never modify it.
- Output: `<input_stem>_messages/` next to the input file, containing
  `README.md`, `general.md`, `infrastructure.md`, and
  `messages/<stable-id>.md` for each logical message/version.
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

### Stage 1 — Format-agnostic overlapping character windows

- Treat source as opaque UTF-8 text. Its format is unknown: do not parse
  Markdown headings, tables, embedded images, or other syntax to split it.
- Split by a strict maximum Unicode character count, including oversized
  individual lines, without breaking UTF-8 characters.
- Adjacent windows overlap by a bounded number of characters (currently 1000).
  Preserve source positions and ensure the full source is covered.
- Keep input and model contexts bounded, including validation and merge steps.
- Duplicates from overlap must be consolidated, not written twice.

### Stage 2 — Extract

- Run the extraction prompt for every chunk, preserving original line ranges.
- Extract complete facts and explicit continuations, not summaries that drop
  fields or constraints.
- Preserve source citations, unresolved references, and partial definitions.
- Represent empty chunks explicitly so they cannot be mistaken for a failure.

### Stage 3 — Merge and normalize by destination file

- Route reviewed fragments to stable `messages/<stable-id>.md` files and
  merge fragments for the same message/version incrementally.
- Consolidate shared facts in `general.md` and helper contracts in
  `infrastructure.md`. Generate `README.md` as an index.
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

### Stage 5 — Write specification directory

- Write `<input_stem>_messages/` only after successful assembly and required
  checks. Stage files in a temporary directory and publish as one directory.
- Use consistent Markdown across `README.md`, `general.md`,
  `infrastructure.md`, and `messages/<stable-id>.md`. Message files contain
  field tables, size/allocation analysis, parsing/validation rules, source
  references, and unresolved questions; infrastructure is specification only.
- Do not overwrite the input. Avoid partial/corrupt output on failures.

## Acceptance criteria / tests

1. Short Markdown and plain-text documents work.
2. Long documents spanning many chunks work without unbounded model context.
3. Tables crossing chunk boundaries retain their headers and every row.
4. A message described across distant sections is merged once, correctly.
5. Shared definitions (e.g. endian and common headers) reach relevant messages.
6. Independent verification detects missing or altered fields.
7. Overlap does not duplicate messages or fields.
8. Output is a Markdown directory with one file per message, shared rules,
   helper contracts, and an index, with traceable source references.
9. Static/bounded memory is preferred when justified; unknown bounds remain
   explicit; helper classes are specified but not implemented.
10. No source code is generated and the input file is unchanged.
11. Build, tests, Clippy, and formatting checks pass.
12. Errors in reading, parsing, model responses, merging, validation, or
    writing are reported rather than silently producing a success result.

## Implementation note

The feature-branch implementation uses overlapping character windows and
per-file incremental merging. It still needs comprehensive integration tests,
robust cross-window identity resolution, hierarchical merge for oversized
single-message specifications, and independent verification of the final
merged files against all contributing source ranges. Do not describe the
current implementation as fully validated.
