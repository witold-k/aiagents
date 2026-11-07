# Semantic coverage audit

Status: initial read-only inventory (2026-11-07). No extraction or identity behavior changed.

## Goal

Measure how much source-backed information survives each stage of `extract_messages`, without treating an unresolved identity as an error or allowing LLM hypotheses to become verified facts.

## Current output contract (schema version 1)

`src/workflows/chunked_transform.rs::write_spec_directory` writes:

- `manifest.json`: source, output paths, overall verification flag and paths to fact/quality files.
- `facts.json`: canonical facts grouped by specification path.
- `quality.json`: diagnostics from `extraction_quality::diagnose`.
- `general.json`, `infrastructure.json`, `messages/*.json` (where present): name, facts, source windows, and verification.

The per-file `verification = supported_quotes` means all included facts have a supported review status; it does **not** establish completeness, statement truth, or correct message ownership. `unresolved` and `no_facts` are separate states.

## Existing read-only diagnostics

- `message_identity::statement_locator_stats`: statement-position diagnostics.
- `message_identity::parameter_coverage_in_document`: matched/unmatched parameter evidence, including lexer kind and section.
- `message_identity::parameter_structure_audit`: unresolved structural hints.
- `llm_structure::semantic_units`: lexically anchored semantic units.
- `llm_structure::hypothetical_units`: separate relation-backed hypotheses, explicitly not verified.
- `llm_structure::tiered_unit_evidence`: disjoint evidence counts across lexical, relational, and outside/ambiguous tiers.
- `llm_structure::remaining_unit_evidence` and `outside_unit_evidence`: classify remaining evidence without assigning ownership.

The LLM structure analysis is opt-in via `AIFIX_LLM_STRUCTURE`; it is not a normal published JSON representation.

## Audit dimensions

Measure these separately, with explicit denominators:

1. **Source coverage**: independently enumerated source assertions vs extracted assertions. This requires a source-derived reference inventory; counting only extracted facts is circular.
2. **Evidence coverage**: extracted assertions with exact, unambiguous source locators vs all extracted assertions. Exact quote location does not prove statement semantics.
3. **Structure coverage**: source row occurrences in a unique lexically anchored unit, a hypothetical unit, or neither. Count duplicate occurrences individually; ambiguous overlaps are not covered.
4. **Identity coverage**: source-supported identities, unresolved identities, and conflicts. Do not turn unresolved into guessed owners.
5. **Publication coverage**: staged canonical facts vs facts present in `facts.json` and per-spec JSON. Compare stable source provenance and review state, not just statement text.

Do not collapse these into one quality percentage.

## First replay procedure

1. Use the same OpenAMIP input and completed checkpoint as the previous replay; keep model/configuration and source hash fixed.
2. Run `AIFIX_IDENTITY_REPLAY=1` and capture all `[CHUNKED]` diagnostics.
3. Enable `AIFIX_LLM_STRUCTURE` only for a separate controlled comparison; it may perform LLM calls, so do not assume this mode is replay-only.
4. Compare the exported `manifest.json`, `facts.json`, `quality.json`, and per-spec files against the captured diagnostics.
5. Record concrete mismatches with source byte offsets, original row/cell evidence, and stage of loss. Classify an unknown cause as **unknown**, not as an extraction failure.

## Invariants for later implementation

- Never infer message identity from a table heading, adjacency, or LLM vote alone.
- Keep lexical anchors, relational hypotheses, and verified evidence distinct.
- Preserve repeated source occurrences and ambiguous matches.
- Do not modify canonical facts, checkpoint state, or output schema in the audit phase.
- Keep `tests -> src` dependency direction.
- Preserve behavior for documents without tables.

## Next implementation candidate (after replay)

If the replay demonstrates a measurable gap, add a small opt-in **read-only** JSON audit report alongside existing outputs, with counts and provenance sufficient to reproduce each classification. Do not alter schema version 1 or the existing published files until a compatibility decision is made.
