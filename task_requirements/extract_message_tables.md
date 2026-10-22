# Task Requirements: extract_message_tables

## Purpose

Extract source-grounded, atomic protocol facts from large UTF-8 Markdown or
plain-text documents. The output is an intermediate specification for later
code generation, **not** generated implementation code.

The generic `chunked_transform` workflow must remain independent of any
particular protocol or test catalogue.

## Current runtime contract (v6)

- Input: exactly one existing `.md` or `.txt` file inside the project.
  Do not modify it.
- Output: `<input_stem>_messages-json/` next to the input, or the sibling
  `-unverified` diagnostic directory when unresolved findings exist.
- Output includes per-destination Markdown projections, `README.md`,
  `facts.json`, and `manifest.json`.
- Destinations: `general.md`, `infrastructure.md`, and
  `messages/<lowercase-slug>.md`.
- Model exchange: one JSON object with a `files` array; each fact supplies
  an atomic `statement` and verbatim `source_anchor`. An empty result is
  `{"files":[]}`. Never use historical FILE / END FILE framing.

## Extraction scope

Record documented message identity, direction, framing, fields, field order,
widths, offsets, encodings, constraints, variants, versions, size bounds,
checksums, errors, and cross-message dependencies where present. Preserve
unknowns and contradictory statements instead of inventing resolutions.

For memory and helper contracts, distinguish fixed/bounded storage from
unbounded or unknown requirements. Never assume dynamic allocation is
necessary. Describe helper responsibilities and error conditions as facts or
explicit unresolved questions, not executable code.

## Deterministic-first processing

1. Mask opaque payloads and divide source into bounded overlapping character
   windows (currently 4,000 characters with 1,000 overlap).
2. Ask the LLM to propose atomic facts only where interpretation of prose or
   tables is necessary.
3. Parse and validate JSON, paths, explicit identities, source anchors, and
   exact duplicates in Rust. Reject conflicting identities deterministically.
4. Persist source-attributed facts and cursors in a v6 checkpoint.
5. Perform a bounded coverage sweep for missing message identities. It is an
   LLM-based omission check, **not** a completeness guarantee.
6. During fact audit, use a deterministic source-quote check whenever possible;
   call the LLM only when evidence cannot be established that way. A quote
   matching the source does not establish semantic correctness.
7. Render Markdown and JSON deterministically. Keep unresolved assertions
   visible only in diagnostic output, not as fully verified specifications.

Every `SourceFact` records statement, source chunk, source window line range,
optional evidence quote, and review status. Exact statement duplicates are
merged; semantically similar or contradictory statements are not silently
collapsed. When a later window supplies evidence for an existing unsupported
fact, update its provenance together with its quote.

## Checkpointing and safety

Use `<input_stem>_messages.v6.checkpoint.json`. Do not migrate historical
checkpoint versions. Bind checkpoints to source content, task requirements,
and the **actual** JSON extraction and coverage prompts. Reject mismatches.
Persist progress after completed windows and fact reviews. Never overwrite
an existing output directory or modify the source document.

## Known limitations

- Verbatim source evidence is necessary but not semantic verification.
- Completeness, distant cross-references, and stable identities for complex
  protocols are not guaranteed.
- Conflicting identities are logged and skipped; no separate structured
  conflict report exists yet.
- The coverage inventory may exceed its bounded context on very large inputs.
- Some unresolved checks still require an LLM call.
- Full build, test, formatting, and Clippy validation must be run before merge.

## Acceptance checks

Short and long Markdown/text inputs, overlap, conflicting identities, malformed
JSON, invalid paths, unsupported anchors, evidence provenance upgrades,
checkpoint resume/rejection, unverified output, and absence of generated code
must be covered by tests. Do not claim end-to-end correctness without running
those checks.
