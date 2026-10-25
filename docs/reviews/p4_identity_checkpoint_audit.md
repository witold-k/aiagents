# Review: P4 identity, checkpoint resume, and audit

Reviewed against branch `refactor/generic-source-provenance` on 2026-10-24. This is a static review; no local Rust tests were executed.

## P1 — Evidence review can be reused for different source occurrences

In `chunked_transform.rs`, the `prior_review` lookup matches only
`source_chunk` and `statement`. Two identical statements from different
source locations within one chunk can therefore reuse a `Supported` review.
The code validates that the previously reviewed quote exists *somewhere* in
the chunk, not that it belongs to the current fact's source occurrence.
This is especially relevant when the same sentence appears in distinct message
sections. **Recommendation:** retain and compare exact absolute evidence
positions, or explicitly treat the review as statement-level only and never
as source-occurrence validation. Test identical statements at two offsets
under different type declarations.

## P1 — Audit cursor invalidation on projection changes

The projection rebuild resets `fact_audit_cursor`, `fact_audit_version`
and `audit`, which is good, but `identity_projection_version` is currently
fixed at `1`. A change in resolver semantics without bumping this value
leaves old projected facts and audits intact on resume. **Recommendation:**
centralize projection and audit version constants, define migration invariants,
and test checkpoint resumes across a version bump.

## P1 — Fact provenance is window-level in published facts

`record_segmented_facts` receives `part.start_line` and
`part.end_line` even when a `CandidateFact` carries an exact byte offset.
As a result, published `SourceFact` line ranges identify the source *window*,
not necessarily the exact evidence span. The checkpoint candidates retain
the more precise offset. **Recommendation:** propagate source byte offsets
and derived exact line ranges into `SourceFact` and its evidence occurrences.
Regression: two facts from distant lines of one window must have different
locations.

## P2 — Replay and publication have separate deduplication implementations

The read-only replay and the normal P4 projection duplicate the
`seen_locations` logic. Replay increments `positioned` before deduplication,
and unpositioned legacy facts are never deduplicated. Although intentional
today, these paths can drift. **Recommendation:** extract one pure deterministic
projection function returning typed occurrences, duplicates, unresolved
diagnostics and per-message counts; use it for both replay and publication.

## P2 — P3 and P4 coexist in extraction and coverage

Both passes populate legacy `checkpoint.facts`/`files` and staged
`checkpoint.candidates`. P4 later clears legacy message facts and rebuilds
them from candidates. This doubles state and creates an intermediate checkpoint
where the two representations disagree. **Recommendation:** define an explicit
staging/projection state machine and enforce it in checkpoint validation.

## P2 — Existing output comparison does not verify individual JSON contents

The existing-output guard compares `facts.json`, manifest metadata and
the existence of each listed file. It does not compare each file's bytes or
parsed content to the expected projection. **Recommendation:** verify the
content of every expected JSON output file, or store and check deterministic
content hashes in the manifest. Test a modified single-message JSON file.

## P2 — Checkpoint filename and schema version diverge

`CHECKPOINT_VERSION = 16` while `checkpoint_path()` uses
`v10.checkpoint.json`. This may be intentional for compatibility, but the
filename suggests the wrong schema version. Document the distinction and test
migration before renaming any existing checkpoint.

## P2 — Missing type is the dominant failure

`resolve_fact_type` in `message_identity.rs` checks only local
explicit labels, twelve-line standalone heading ownership, and contiguous
table blocks. It has no document-wide definition index. The OpenAMIP replay
reported 543 occurrences with `no_source_type`. **Recommendation:** add
read-only diagnostics grouped by source section before widening ownership
rules; require explicit definition boundaries and ambiguity reporting.

## Test matrix (not yet executed)

1. Two identical statements at different offsets under different type declarations.
2. Exact source lines preserved through overlap and publication.
3. Replay and publication return identical projection counts and identities.
4. Resume with changed projection version invalidates stale audit without
   rerunning extraction.
5. Interrupted recovery resumes without duplicate or missing candidates.
6. Tampered individual message JSON is detected even when `facts.json` is unchanged.
7. Legacy checkpoint with missing offsets remains conservative.
8. Case-sensitive `Type: A` and `Type: a` never coalesce.

## Suggested order

First add pure projection and provenance tests; then unify replay and normal
projection; then fix audit reuse and version gates. Do not broaden type
inference until the invariants are tested.
