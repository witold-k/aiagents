# Chunked extraction — current implementation

> Experimental workflow. Source quotations and structural diagnostics do **not** establish semantic correctness or completeness.

## Purpose

`aifix -t extract_message_tables -a document.md` extracts candidate protocol assertions from overlapping document windows. Rust manages source attribution, checkpoints, identity projection and JSON publication; an LLM proposes assertions. The workflow does not generate protocol code.

## Source and candidate processing

- The source is processed in document order, with bounded overlapping windows.
- `simplelexer` supplies source-table information before normalization. Table structure is supplementary; documents without tables must remain supported.
- P4 stages `MessageCandidate` entries and source-attributed facts before resolving identities. An extracted filename, example or nearby mention is not proof of a message type.
- Identity handling is case-sensitive. Unresolved candidates remain available for investigation rather than being assigned to a guessed message.
- Evidence quotations are checked against source text, but an exact quotation alone does not validate the full assertion.
- Coverage and fact review are separate from extraction. The runtime prompt constants in `src/workflows/chunked_transform.rs` define the extraction/coverage exchange format. The task description in `task_descriptions/tasks/extract_message_tables.md` still contains older Markdown-output and `source_anchor` wording; do **not** treat those portions as the active wire format. Changing that task description can change the extraction fingerprint and invalidate existing checkpoints.

> Detailed window boundaries, recovery behavior and scoped context design: [Window and context README](window_context/README.md).

## Window sizing and LLM context

The P4 source window target is currently `CHUNK_CHARS = 8_000` Unicode characters, with `CHUNK_OVERLAP_CHARS = 1_000` characters repeated across adjacent windows. Windows are produced in source order; the overlap protects boundary-spanning text, but does not constitute a persistent document-level semantic context.

The CLI option `-k` / `--tokens` (default: 30,000) configures the maximum LLM **output** tokens. P4 extraction, coverage, their retries, and structure inference now use the configured cap rather than a fixed 2,048-token constant. A small classification request remains capped at `min(configured_tokens, 512)`. This output budget is independent of the input window size.

The first extraction pass supplies source-window text and limited preceding source context for recovery windows; it does **not** prepend a comprehensive deterministic document-state summary to every request. The later coverage pass additionally supplies an inventory of known message paths. The full-document identity context is used for deterministic identity processing, but is not automatically serialized into every extraction prompt. A reusable, source-grounded per-window context is a **proposed refactor**, not current behavior. Keep source-derived context distinct from unverified LLM hypotheses.

Changing chunk size or overlap can alter chunk numbering, source-window-relative offsets, and checkpoint compatibility. For before/after quality comparisons, prefer fresh runs rather than assuming old checkpoints remain comparable.

## Internal data and checkpoint responsibilities

The following types live in `src/workflows/chunked_transform.rs`:

| Type | Current role |
| --- | --- |
| `ExtractionCheckpoint` | Serialized progress and intermediate results for extraction, coverage, recovery, audit, and identity projection |
| `SourceFact` | Deduplicated assertion plus an initial source location, optional quotation, additional occurrences, and review status |
| `FactEvidenceOccurrence` | One source occurrence, with chunk index, line range, optional quotation, and optional byte offset **within that source chunk** |
| `FactReview` | `pending`, `supported`, or `unresolved` review state |
| `FactEvidence` | Strict JSON response shape (`evidence_quote`) for a fact-quotation check |
| `ExtractedFact`, `ExtractedFile`, `ExtractedWindow` | `#[cfg(test)]` legacy/test-only extraction JSON shapes, including `source_anchor`; not the active segmented extraction contract |
| `ExtractedRecords` | Intermediate `Vec<(path, Vec<(statement, optional_quote)>)>` representation |
| `EvidenceSegment` | Numbered source segment with text, line index and byte offset relative to its source window |

`ExtractionCheckpoint` contains several distinct responsibilities:

- **Identity and compatibility:** `version`, `source_fingerprint`, `fact_audit_version`, `identity_projection_version`.
- **Progress:** `next_chunk`, `coverage_next_chunk`, `audit`, `fact_audit_cursor`.
- **Accumulated results:** `files`, `facts`, `source_windows`, `candidates`, `unresolved_findings`.
- **Interrupted-window recovery:** `recovery_chunk`, `recovery_pending`, `recovery_records`, `recovery_candidates`.

`migrate_identity_projection_version()` recognizes a legacy checkpoint encoding: when the projection version is zero and the fact-audit version equals 7, it records projection version 1. This is compatibility logic, not an extra LLM pass.

A `SourceFact` stores its primary source location directly **and** a list of `evidence_occurrences`. This is a candidate for consolidation, but the migration must preserve every distinct occurrence and all review/provenance information. `files` contains rendered per-path content while `facts` contains structured facts; whether the rendered copy can be derived safely on resume requires call-site and checkpoint analysis. Similarly, `ExtractedRecords` and candidate buffers may represent different pipeline stages rather than redundant information.

`verified_fact_quote()` parses the strict `FactEvidence` JSON and checks the quotation against the attributed source using `resolve_source_quote()`. Finding the quotation establishes source presence, **not** semantic validity of the statement. `FactReview::Supported` must not be presented as independent semantic verification.

### Refactoring questions (not implemented)

1. Trace the actual transformations `EvidenceSegment → LLM response → ExtractedRecords / MessageCandidate → SourceFact → files`; identify where information is copied, lost, or reconstructed.
2. Prefer one explicit collection of source occurrences per fact, if compatible with persisted checkpoints and existing consumers.
3. Prefer absolute, source-grounded positions over chunk-relative coordinates where possible; keep original-source and normalized-source coordinate systems distinct.
4. Review whether rendered `files` and intermediate recovery buffers must be persisted alongside structured facts.
5. Move legacy `#[cfg(test)]` response models to test-owned code if no production dependency requires them, maintaining tests → src and never src → tests.
6. Introduce deterministic per-window context only after defining its source, boundaries, and behavior for tableless documents; retain the 1,000-character overlap unless tests justify changing it.

### Context lifetime and reset boundaries (design requirement; not implemented)

Context must be **scoped**, not an ever-growing history. Distinguish document/section context from the currently active message-definition context. Retain relevant source-grounded section information across windows, but expire message-specific context when source structure establishes that the definition has ended or a new independent definition has begun. Chunk boundaries alone must never reset message context; a message boundary may also occur inside one chunk.

Derive context transitions deterministically from document order, structural boundaries and source positions (including simplelexer information where available). An LLM-suggested boundary is a hypothesis, not sufficient evidence for irrevocably discarding context. When boundaries are ambiguous, preserve the uncertainty rather than silently treating the next window as a continuation or a new message. The 1,000-character raw-source overlap remains separate from semantic context lifetime.

Tests should cover a message spanning windows, two messages in one window, a clear message termination, ambiguous boundaries, and tableless documents. Avoid adding a separate HTML parser or source reconstruction in aiagents.


## Checkpoints and resume

The current code uses `CHECKPOINT_VERSION = 16` in `src/workflows/chunked_transform.rs`. The checkpoint filename is managed separately by `checkpoint_path()` and currently ends in `.v10.checkpoint.json`. This is **not** the schema version; the stored version is 16.

A completed checkpoint can be inspected without rerunning the model:

```sh
AIFIX_IDENTITY_REPLAY=1 aifix -t extract_message_tables -a document.md
```

Replay is diagnostic: it does not publish output or establish verified identities. A normal run can also resume a completed checkpoint and report diagnostic counters before detecting an already published destination.

## Provenance and structure diagnostics

The current workflow reports:

- `SOURCE FACT INVENTORY`: candidate fact occurrences and distinct statement/evidence pairs.
- `IDENTITY CLASSIFICATION`: explicitly typed and unresolved candidates.
- `STATEMENT LOCATORS`: exact source-string positions, missing locations and mismatches.
- `STATEMENT STRUCTURE`: deterministic comparison between exact statement positions and evidence anchors, distinguishing same/different table rows, section-only matches and unknown cases.
- `P4 UNRESOLVED DIAGNOSTICS`, structural triage and parameter inventory.

These counters describe *source locations and classification behavior*, not semantic correctness. In particular, `different_segment` in the locator inventory means different byte offsets; it does not prove different table rows or different messages. A shared section does not establish shared message identity. Diagnostic parameter assertions are not automatically published as typed parameters.

## JSON output and verification boundary

The workflow publishes JSON specifications in an `<input-stem>_messages-json` directory, or an `-unverified` diagnostic destination where appropriate. The JSON output uses schema version 1 and includes `facts.json`, `manifest.json`, and per-topic JSON files such as `general.json`, `infrastructure.json` and `messages/<slug>.json`.

Publication is staged and does not silently overwrite an existing output directory. A matching existing output may be recognized on resume. Published facts retain review and source attribution metadata; window-level attribution must not be mistaken for an exact statement span.

`supported` means that source quotation checks succeeded, **not** that the assertion is semantically valid. Treat output as unverified until independently checked against the protocol source.

## Implementation and related documentation

- [Implementation](../src/workflows/chunked_transform.rs)
- [Identity and source-locator implementation](../src/workflows/message_identity.rs)
- [Task contract](../task_descriptions/tasks/extract_message_tables.md)
- [Task overview](tasks/extract_message_tables/README.md)
- [Architecture diagrams](architecture_diagrams.md)
- [Historical project notes](project_status_history.md)
- [Historical P4 review](reviews/p4_identity_checkpoint_audit.md)

Historical checkpoint schemas, Markdown-output stages and older OpenAMIP measurements are deliberately omitted from this current-behavior reference.
