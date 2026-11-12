# TODO

This is a hobby project — the journey is the goal. This file is not intended as
a strict roadmap. It is a reminder of interesting or necessary places to resume
work after a pause.

## When returning to this project

1. Run `just rbuild` and the Rust test/format/lint checks on the current branch; record any failures.
2. Triage 114 missing exact statement locators from the supplied checkpoint by cause, without treating paraphrases as missing source evidence.
3. Complete deterministic scoped-context transitions and document-wide definition ownership, retaining unresolved cases.
4. Evaluate semantic field definitions against multiple independent documents before designing a verified generator input.
5. Continue standalone BLT design, release-document quality, or indexing experiments as time permits.

See [current status](docs/current_status.md) for a dated checkpoint snapshot and [architecture](docs/architecture_diagrams.md) for trust boundaries.

---

# Protocol extraction: current state and next steps

The supplied checkpoint replay reports **25 message candidates**, **238 source-attributed facts**, **124 exact statement locators**, **114 missing exact statement locators**, and **78 source-backed parameter assertions**. It is not a fresh run of the latest branch. These metrics are diagnostic, not semantic correctness or completeness scores.

Implemented diagnostic foundations:
- [x] Retain source-grounded facts, unresolved case-sensitive message identities and statement-locator diagnostics.
- [x] Reuse `simplelexer` original/normalized table evidence and exact source positions.
- [x] Preserve LLM-voted row-continuation units and separate hypothetical units without assigning owners.
- [x] Recognize explicit lexical parameter declarations separately from example assignments.
- [x] Export unverified lexical assemblies, uninterpreted reconstructed table entries and unassembled semantic rows.
- [x] Export evidence-coverage, semantic-review and codegen-readiness diagnostics.
- [x] Remove the protocol-shaped bare-column repair heuristic; keep the extractor generic.

Next, in priority order:
- [ ] Verify the current branch build, formatting, tests and clippy; record actual outcomes.
- [ ] Classify missing locators into paraphrase/normalization, repeated statement, unsupported quote and genuine missing evidence.
- [ ] Complete deterministic scope changes **within** a source window and across window boundaries, using only explicit source declarations.
- [ ] Separate source-verified names, types, units, ranges, optionality and conditions from competing or incomplete evidence.
- [ ] Resolve message/parameter ownership only from explicit source support; preserve unknowns and contradictions.
- [ ] Benchmark table, prose, conditional and damaged-conversion cases from multiple unrelated documents.
- [ ] Define a stable versioned **verified** schema, distinct from the current diagnostic `protocol-model.json`; only then consider a separate C++/Rust generator.

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
