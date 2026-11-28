# TODO

This is a hobby project — the journey is the goal. This file is not intended as
a strict roadmap. It is a reminder of interesting or necessary places to resume
work after a pause.

## Current milestone: stabilization before merge

Feature freeze: finish validation and documentation before adding new semantic features. See [merge gates](docs/current_status.md#merge-gates).

- [ ] Audit all changed production paths and prompts, including stale diagnostic labels and optional failure handling.
- [x] Relocate `tests/workflow_configuration.rs` into mirrored test locations.
- [ ] Audit existing inline tests separately.
- [ ] Run build, all-target tests and clippy on the final candidate; record results.
- [ ] Run fresh comparable extraction on master and the candidate in separate output directories, with matched source/model/settings.
- [x] Restore independent parameter inventory and validate coverage audit in checkpoint replay.
- [ ] Record enum-link regression and evidence limitations in the merge decision; verify terminal failures and incompatible checkpoint preservation.
- [ ] Record the merge decision and remaining limitations; no automatic PR or merge.

## When returning to this project

1. Run `just rbuild` and the Rust test/format/lint checks on the current branch; record any failures.
2. Triage missing exact statement locators from the supplied checkpoint by cause, without treating paraphrases as missing source evidence.
3. Complete deterministic scoped-context transitions and document-wide definition ownership, retaining unresolved cases.
4. Evaluate semantic field definitions against multiple independent documents before designing a verified semantic schema.
5. Continue standalone BLT design, release-document quality, or indexing experiments as time permits.

See [current status](docs/current_status.md) for a dated checkpoint snapshot and [architecture](docs/architecture_diagrams.md) for trust boundaries.

---

# Protocol extraction: current state and next steps

An older checkpoint replay reports **25 message candidates**, **238 source-attributed facts**, **124 exact statement locators**, **114 missing exact statement locators**, and **78 source-backed parameter assertions**. It is not a fresh run of the latest branch. These metrics are diagnostic, not semantic correctness or completeness scores.

The 2026-11-27 checkpoint replay reports 18 candidates, 226 facts, 96 exact statement locators, 130 missing exact locators, 57 parameter entities, 78 linked assertions and 109 unresolved ownership occurrences. This is a resumed replay, not a fresh extraction or a semantic correctness score.

The 2026-11-28 replay retains 291 candidate facts, 289 distinct statement/evidence pairs, 201 parameter assertions (78 linked, 123 unresolved), and 31 enum members without linked contexts. Coverage audit schema 2 passes without the previous count warning. See the current status for limitations.

Implemented diagnostic foundations:
- [x] Retain source-grounded facts, unresolved case-sensitive message identities and statement-locator diagnostics.
- [x] Reuse `simplelexer` original/normalized table evidence and exact source positions.
- [x] Preserve LLM-voted row-continuation units and separate hypothetical units without assigning owners.
- [x] Recognize explicit lexical parameter declarations separately from example assignments.
- [x] Export unverified lexical assemblies, uninterpreted reconstructed table entries and unassembled semantic rows.
- [x] Export evidence-coverage and semantic-review diagnostics.
- [x] Remove the protocol-shaped bare-column repair heuristic; keep the extractor generic.

G8 domain independence (migration scope only):
- [x] Internal case-sensitive string identifiers in deferred identity resolution.
- [x] Shared readable UTF-8 hex path encoding; no single-letter exceptions in new deferred projections.
- [x] Explicit extraction identifier validation independent of output filenames.
- [x] Diagnostic declaration-column consensus with exact cell evidence; no row-level promotion.
- [x] Diagnostic table-role hypotheses with source-cell validation and bounded sampling.
- [ ] Audit production code and generic prompts for remaining document-specific assumptions.
- [ ] Replace fixed table headers and section-name/example heuristics with source-backed role evidence; preserve unresolved cases.
- [ ] Retire remaining active single-letter legacy path and identity adapters; reject incompatible checkpoints without migration.
- [ ] Validate case-sensitive arbitrary identifiers and unrelated document structures in synthetic tests and independent document runs.
- [ ] Complete migration only after remaining active assumptions are removed and regression checks pass.

G9 generic document reconstruction (separate from migration):
- [x] Diagnostic delimiter integrity and continuation-graph evidence, separate from LLM votes.
- [ ] Integrate table roles, identifier boundaries and structural relation hypotheses.
- [ ] Reconstruct logical rows with original-source provenance, conflict handling and multi-row chains.
- [ ] Evaluate damaged conversion cases and expose validated units to semantic extraction.

Separate infrastructure and semantic work:
- [x] Stop retries on explicit workflow errors and propagate failure exit status.
- [x] Validate eight extraction workflow boolean flags before processing.
- [x] Remove checkpoint identity migration and reject old checkpoint schemas (v17).
- [ ] Verify table-role call failure propagation and audit remaining LLM call sites and boolean flags outside the extraction workflow.
- [x] Restore parameter assertion linking in the supplied replay (78 linked).
- [ ] Investigate missing enum assertion coverage and linking in the next development cycle; retain structural inventory.
- [x] Diagnostic reference-gap classification and conservative candidate equivalence grouping.
- [ ] Complete reference-gap resolution and candidate equivalence verification before promoting a verified semantic IR.

Next, in priority order:
- [ ] Verify the current branch build, formatting, tests and clippy; record actual outcomes.
- [ ] Classify missing locators into paraphrase/normalization, repeated statement, unsupported quote and genuine missing evidence.
- [ ] Complete deterministic scope changes **within** a source window and across window boundaries, using only explicit source declarations.
- [ ] Separate source-verified names, types, units, ranges, optionality and conditions from competing or incomplete evidence.
- [ ] Resolve message/parameter ownership only from explicit source support; preserve unknowns and contradictions.
- [ ] Benchmark table, prose, conditional and damaged-conversion cases from multiple unrelated documents.
- [ ] Define a stable versioned **verified** schema, distinct from the current diagnostic `protocol-model.json`..

Keep document-administration filtering lower priority unless it demonstrably changes semantic quality. Do not convert a vote or an exact quotation into a verified definition.

---

# BLT: independent build/lint/test and diagnostics module

Explore extracting BLT from the LLM repair workflow into a standalone, LLM-free Rust module (possibly a separate crate later). Keep execution, diagnostic preparation and result access independent of any agent or prompt implementation.

- [ ] Define a small BLT API for build-system detection, build/lint/test execution, stage status, exit codes, and complete process output. Distinguish failed, successful, skipped and execution-error stages.
- [ ] Capture complete output and prepare useful diagnostic locations/context without requiring a universal, language-specific `DiagnosticReport` schema. Preserve raw output and provenance; add language-specific extraction only where reliably useful.
- [ ] Provide bounded reading/search of stage output and a way to compare failures between runs, without making the LLM responsible for facts BLT already knows.
- [ ] Keep LLM-facing specialized diagnostic tools in an adapter in `aiagents`, not inside BLT; reuse the existing tool parser/execution infrastructure.
- [ ] Refactor `BLTWorkflow` to orchestrate repair using BLT results rather than owning process execution and diagnostic preparation.
- [ ] Add deterministic tests for output capture, stage skipping/failure, large logs, and repeated versus changed failures; benchmark the repair workflow on real Rust and C++ failures before expanding diagnostic parsing.
- [ ] Decide whether to extract a standalone crate only after the module API and use cases stabilize.

Design principle: BLT supplies verifiable facts and accessible evidence; the LLM interprets causes and proposes edits; BLT independently validates the outcome. Avoid premature diagnostic schemas and additional tool frameworks.

---

# Context and Source Analysis

## AST

For Rust source analysis, use the Rust `syn` crate rather than an external AST
binary.

---

# Future Ideas

## Local indexing and search

Experiment with indexing source code and documents so useful context can be
found without simply loading more files into the LLM context.

Possible directions include:

- keyword or LLM-assisted indexing of text blocks
- human-oriented search
- LLM-oriented context retrieval
- persistent local indexes
- combining exact/full-text retrieval with semantic methods

## TokenDB

Explore whether `token_db` is useful as a persistent vocabulary/corpus layer
for indexing and retrieval experiments.

## Embeddings and SVD

Investigate semantic retrieval techniques such as embeddings and
singular-value decomposition (SVD).

Possible experiments include:

- normalized sliding windows over groups of words
- storing reduced representations
- retrieving similar problems and solutions
- decomposing a problem into several steps
- comparing simple lexical retrieval, SVD/LSA, and embeddings on real data

## Release documentation

The basic Git-history-to-`RELEASE_NOTES.md` workflow exists. Interesting future
work is therefore about quality rather than merely implementing it:

- decide which Git context is actually useful to the model
- evaluate summaries on real release ranges
- keep generated release notes concise and technically meaningful
- consider whether diffs, complete files, or a mixture provide better context

## Web access

- use Lightpanda for web lookup (maybe, if ever ...)
[Lightpanda](https://lightpanda.io/)

---

# Interactive Mode

An interactive mode remains an idea for allowing users to work with the agent
conversationally during a development session. It is not necessarily the next
thing to implement.

The intended workflow could be:

- give the agent a task
- inspect what the agent is doing
- provide additional instructions
- review proposed changes
- allow or reject actions
- continue the task interactively
- inspect build and test feedback

The initial implementation should use Markdown documents rather than
introducing another dedicated UI or protocol.

A Markdown document can act as the interaction surface:

    User/editor
        |
        v
    Markdown document
        |
        v
    file change detected
        |
        v
    agent receives new text
        |
        v
    agent continues

The editor only needs to support normal file editing and saving.

An external watcher/tool can detect changes and forward the updated text
to the agent.

A simple convention, such as a keyword at the end of the Markdown document,
can indicate that the user's request is complete and should be processed.

This keeps interactive mode consistent with the project's goal of using
simple existing mechanisms rather than adding another tooling layer.

- [ ] Extend document integrity diagnostics with mapped original/normalized cell comparisons, cross-row delimiter analysis, additional entity encodings and explicit provenance for any lossless repairs. Keep suspected damage separate from proven conversion loss.

- [ ] Integrate delimiter continuation evidence into the existing structural relation graph without counting deterministic observations as LLM votes. Validate logical column alignment, map both cells to original source, and test reconstruction across unrelated tables before promoting any candidates.

- [ ] Extend the continuation graph projection to complete logical rows with original-source cell spans, multi-row chains and explicit conflict resolution; only then expose validated units to entity extraction.
