# Current project status — 2026-11-28

[Overview](../README.md) · [Architecture](architecture_diagrams.md) · [Extraction](chunked_transform.md) · [History](project_status_history.md) · [TODO](../TODO.md)

## Semantic model and generic document understanding — foundation

Stabilization candidate on `feature/semantic-reference-resolution`. The pre-documentation GitHub comparison against master (base `3fd463f`) showed 129 commits ahead, zero behind and 29 changed files. These are historical comparison counts, not the current branch size. No merge or PR is authorized by this status document.

## Implemented foundations

| Area | Implemented | Remaining boundary |
| --- | --- | --- |
| Runtime | Terminal workflow errors and request failures stop retry loop; failure exit status | Audit remaining CLI and LLM failure paths |
| Configuration | Eight strict extraction boolean switches | Other switches not audited |
| Checkpoints | Schema 17; incompatible checkpoints rejected, no migration | Practical rejection and resume tests |
| Identity | Case-sensitive strings; shared readable UTF-8 hex paths; explicit identifiers validated independently of paths | Legacy declaration, example and table heuristics remain |
| Source model | Original and normalized spans, parameter/enum inventories, document-scoped registry | Ownership and semantic truth not established |
| References | Candidate bindings, gap reasons, conservative equivalence grouping | Verified references, conflict resolution and promotion |
| Tables | Bounded table-role hypotheses and exact cell validation; agreed declaration-column candidate | Row-level declarations and ownership not inferred |
| Integrity | Mixed/unmatched delimiter diagnostics and adjacent-cell continuation candidates | No automatic repair or verified logical reconstruction |

All semantic and structural proposals remain diagnostic. A unique candidate, matching quotation or agreeing LLM attempts do not prove ownership or semantic correctness.

## Latest checkpoint replay — 2026-11-28

Completed schema-17 checkpoint, resumed at chunk 24/23. Existing unverified output preserved. Coverage audit schema 2 completed without the inconsistent-count warning.

| Metric | Observed |
| --- | ---: |
| Candidates / explicitly typed / unresolved | 17 / 2 / 15 |
| Candidate facts / distinct statement-evidence pairs | 291 / 289 |
| Exact / missing statement locators | 93 / 198 |
| Exact evidence for unresolved candidate facts | 251 / 251 |
| Original mapping: exact / structural / missing | 202 / 68 / 21 |
| Parameter entities | 57 |
| Independent parameter assertions | 201 |
| Linked / unresolved parameter assertions | 78 / 123 |
| Enum members / linked contexts | 31 / 0 |
| Valid LLM structure attempts | 3 / 3 |

The parameter inventory is independent of message identity. Unresolved-message parameter coverage is a separate occurrence partition: 102 = 78 matched + 6 unmatched + 18 other. Neither exact evidence nor linking proves semantic truth. Enum definitions remain available, but missing assertion coverage/linking is a known regression deferred to the next development cycle rather than expanding the current feature freeze. The replay is not a matched fresh comparison against master.

## Historical extraction observations

The following observations were reported before the final schema-17 changes. They are not a controlled A/B experiment or fresh validation of the current head.

| Metric | Earlier replay (Nov 27) | Later extraction (Nov 27) |
| --- | ---: | ---: |
| Source-attributed facts | 226 | 249 |
| Exact statement locators | 96 | 73 |
| Missing exact statement locators | 130 | 176 |
| Parameter entities | 57 | 57 |
| Linked parameter assertions | 78 | 0 |
| Linked enum members | 14 | 0 |
| Unresolved candidate ownership occurrences after projection | 109 | 33 |

The later run also reported 106 rejected extraction entries and 219 rejected coverage entries; these counts may overlap and do not directly measure lost facts. Publication failed because existing output differed from the checkpoint. Reduced unresolved counts do not demonstrate improvement: parameter or enum evidence may have been misassigned. The earlier master-era Nov 25 replay reported 689 occurrences, 301 exact and 365 structural original-source mappings and 103 linked parameter assertions; different run configurations prevent direct comparison.

The earlier 25-candidate/238-fact snapshot (124 exact locators, 114 missing) is retained here for historical context. Denominators and coordinate spaces differ across reports.

## Merge gates

- [x] Align reserved-marker classifier labels between identity and structural analysis.
- [ ] Finish reviewing the full code diff, legacy paths, report behavior and error propagation.
- [x] Relocate the configuration and terminal-error tests into mirrored modules.
- [ ] Verify test discovery and coverage after relocation.
- [ ] Record final `just rbuild`, `cargo test --all-targets` and clippy results. Earlier incremental builds/tests were reported successful by the user; the final candidate has not been validated here.
- [ ] Run matched fresh extractions on master and candidate using the same input bytes, model/version, settings and comparable cache policy. Use separate work directories and output paths. Do not reuse or migrate old checkpoints.
- [ ] Compare source-backed facts, rejected entries, exact/structural/missing mappings, parameter and enum links, unresolved identities, conflicts, call counts and publication outcome. Inspect actual evidence and representative records, not only counts.
- [x] Restore parameter linking and confirm corrected coverage audit in replay.
- [ ] Accept or reject the documented enum-link regression explicitly; check terminal errors, nonzero exit and preservation of incompatible checkpoints and existing outputs.
- [ ] Record remaining limitations and explicit merge authorization.

## Next work after stabilization

G8: finish generic declaration/ownership discovery and retire active protocol-shaped heuristics; validate unrelated documents. G9: reconstruct logical rows with original provenance. P6.12: semantic reference verification. P6.13: independently validated semantic IR.

## Documentation conventions

Keep historical results distinct from current measurements. Label Rust validation, LLM proposals and future stages explicitly. Never infer semantic correctness from build success or a single replay.
