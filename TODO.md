# TODO

This is a hobby project — the journey is the goal. This file is not intended as
a strict roadmap. It is a reminder of interesting or necessary places to resume
work after a pause.

## When returning to this project

1. For `extract_message_tables`, turn existing source-backed table/semantic-unit evidence into a language-neutral protocol model, without guessing ownership or types.
2. Evaluate that model against the OpenAMIP checkpoint and its semantic-review findings before expanding extraction heuristics.
3. Define an LLM-independent BLT module for build/lint/test execution and diagnostic preparation; then evaluate the repair workflow on real failures.
4. Revisit release-documentation quality, or pick an exploratory experiment: indexing/search, TokenDB, SVD/embeddings, or interactive mode.

---

# Protocol extraction: current state and next steps

`extract_message_tables` currently retains source-attributed candidates and facts without merging uncertain identities. The recent OpenAMIP replay reported **25 candidates**, **238 facts**, **78 distinct source-backed parameter assertions**, and **39 distinct parameters across three sections**. These are observations from one checkpoint, not verified schema coverage.

Completed diagnostic foundations:
- [x] Preserve unresolved message identities and source evidence; inspect statement locations separately.
- [x] Use normalized `simplelexer` tables and source-row spans as supplementary structure.
- [x] Global source evidence coverage and bounded semantic-review queue, with optional LLM verification.
- [x] Codegen-readiness prerequisite inventory (`coverage-audit.json.codegen-readiness.json`).
- [x] Initial language-neutral **hypothesis** model (`coverage-audit.json.protocol-model.json`): assertions, quote anchors, lexical `key=value` hints. The recent replay found **14 lexical hints in 10 candidates**; these are not 14 verified parameter definitions.

Next, in priority order:
- [ ] **Reuse existing evidence:** join normalized table cells and proposed continuation/heading links with the diagnostic model, keeping exact source positions and hypothesis status. Do not add a second HTML parser or table reconstruction pipeline.
- [ ] Distinguish *parameter definitions* (e.g. `statusCode=<int 0 to 65535>`) from *example assignments* (e.g. `statusCode=20`). Record separately supported name, type, range, optionality and conditions, each with provenance; never invent missing properties.
- [ ] Resolve message/parameter ownership only with explicit source support; retain unresolved or competing hypotheses and case-sensitive identities.
- [ ] Cross-check the resulting model against the 78 source-backed parameter assertions and the technical semantic-review gaps. Measure semantic/codegen readiness, **not** merely line coverage or lexical hint counts.
- [ ] Define a stable, versioned, language-neutral **verified** protocol JSON contract for a **separate** C++/Rust generator, including explicit unknowns and validation; defer parser/serializer/state-machine generation until supported.
- [ ] Add regression tests for table continuations, assignment-vs-definition, contradictory evidence, and tableless documents; evaluate on more than one document.

Keep document-administration filtering low priority unless it demonstrably affects model correctness. The present codegen-readiness report is a conservative inventory, not a schema validator; the protocol-model report contains hypotheses, not validated wire definitions.

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
