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
