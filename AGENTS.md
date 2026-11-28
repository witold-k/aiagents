# AGENTS.md

## Scope and purpose

These instructions apply throughout the repository. AIAGENTS is a general-purpose agent system. Technical specification analysis is an application, not an assumption that may leak into the generic core. Keep the repository roadmap and documentation limited to this project's own responsibilities; do not introduce downstream application projects or their integration plans.

## Git workflow

- Work on the designated feature or experiment branch. Never commit to master without explicit permission.
- **Never create a pull request (PR) unless the user explicitly authorizes it.**
- **Never merge branches or pull requests unless the user explicitly authorizes the merge.**
- Permission to implement, commit, push or test changes does not imply permission to create a PR or merge. A successful build or passing tests does not imply authorization either.
- Preserve existing user changes. Do not force-push or rewrite history without authorization.
- Keep changes focused and report commit IDs, outstanding work and verification results.

## Domain independence

- OpenAMIP is a regression input, not an architectural specification. Do not hard-code its names, commands, identifiers, table layouts, section headings or document-specific semantics in production code or generic prompts.
- Do not disguise document-specific rules as generic heuristics. In particular, do not assume single-letter message identifiers, filename-derived identities, fixed English table headers, fixed column positions or reserved-marker syntax.
- Identifiers are opaque, case-sensitive strings unless the source explicitly establishes other semantics. Names alone do not establish identity.
- Protocol-specific semantics must be derived from source evidence or an explicitly selected domain adapter, not silently inferred from the regression corpus.
- Keep Simplelexer independent of application semantics. Lexical and structural observations are not semantic classifications.
- Use synthetic fixtures and multiple unrelated document formats and protocols to validate generality. Do not tune exclusively against OpenAMIP.

## Architecture and evidence

- Separate source structure, semantic hypotheses, entity/reference resolution, and verification.
- Prefer deterministic Rust for parsing, source mapping, validation, indexing and reproducible transformations. Use LLMs for semantic interpretation where deterministic evidence is insufficient.
- Preserve original source spans, quotes, provenance and occurrence multiplicity. Never silently discard source-backed facts merely because identity or ownership is unresolved.
- Treat ambiguous or missing identities explicitly. A unique candidate is not a verified identity. Never merge entities or assertions on name similarity alone.
- Equivalence grouping must preserve conditions, scope, modality, negation, conflicts and all evidence. Do not promote diagnostic hypotheses into verified facts.
- Keep diagnostic reports separate from authoritative outputs. An LLM judgment is not a formal proof.

## Implementation and regression policy

- Inspect existing code, README, TODO and relevant tests before changing architecture.
- Reuse existing string helpers in `crate::utils::stringutils` (including `strip_code_fences`) rather than duplicating string processing in workflows. Inspect the existing utility API first; add reusable missing helpers there with mirrored tests under `tests/`, not in application-specific modules.
- Remove obsolete assumptions at their source rather than adding compensating special cases. Distinguish active production paths from diagnostic and legacy compatibility paths.
- Do not remove a working feature or evidence path without accounting for its replacement and regression impact.
- **Never migrate checkpoints.** Do not implement automatic or manual checkpoint migration, schema upgrades, or conversion of old checkpoint contents. Resume only checkpoints compatible with the current workflow and schema. Version incompatible changes explicitly, reject incompatible checkpoints immediately with a clear error, and require a fresh run. Preserve existing checkpoint files; never silently overwrite or delete them.
- Keep cache and replay compatibility where feasible, without migrating checkpoints.
- Avoid unnecessary LLM calls, context growth and memory consumption. Measure changes against comparable replay inputs.
- Mirror the production source tree under `tests/`: map `src/<relative path>` to the corresponding test location under `tests/<relative path>`. For example, tests for `src/workflows/protocol_model.rs` belong in `tests/workflows/protocol_model_test.rs`, following existing naming conventions.
- Keep test implementations outside `src/`. Preserve existing test discovery and coverage when relocating tests; follow the established module wiring until it can be removed safely.
- Add positive, negative and ambiguity cases. Test evidence preservation and deterministic output, not just aggregate counts.
- Run `just rbuild` and `cargo test --all-targets` when the environment permits. Run relevant replay comparisons for semantic pipeline changes. Never claim unexecuted checks passed.
- Do not treat a successful build or one successful document replay as proof of semantic correctness or generality.

## Documentation and reporting

- Update README and TODO when behavior, architecture or roadmap changes.
- Report actual changes, tests run, limitations, regressions and decisions made without explicit guidance.
- Clearly distinguish implemented behavior from proposals and historical measurements.
- If execution or repository access is unavailable, state the blocker and provide reproducible verification commands.

## Current cleanup direction

The G8 cleanup must audit and remove remaining OpenAMIP-derived assumptions in message identity, parameter and table classification, output naming, diagnostic inventories and prompts. Preserve source evidence throughout. This is outstanding work, not a claim that domain independence has already been achieved.

## Document integrity

- Audit source integrity before semantic interpretation. Distinguish pre-existing source anomalies from transformation-induced damage using mapped source evidence. Never silently repair ambiguous syntax or treat advisory findings as proven corruption.
- Infrastructure failures must not masquerade as semantic uncertainty or successful completion. Preserve checkpoints and allow genuinely offline replay; propagate fatal required LLM failures to the caller.
