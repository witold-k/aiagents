# Extraction quality audit — protocol-neutral lessons

This historical baseline evaluates an unverified extraction export, not the current read-only identity replay. It is **not** a semantic completeness or code-generation readiness claim.

## Failure modes

- A quote can be source-supported while its extracted statement is incomplete, non-atomic, or stripped of conditions.
- Flattened PDF/HTML tables can concatenate parameter types, names, units, and descriptions.
- Examples and parameter definitions must remain distinct; literal assignments are not sufficient type evidence.
- Positional fields, optional indexed keys, and conditional status fields require separate source-backed representations.
- Source-window proximity does not establish message or parameter ownership.
- The export and a later replay may represent different processing stages; their counts are not directly comparable.

## Generic acceptance criteria

1. Preserve case-sensitive identities and exact source evidence; unresolved identities remain unresolved.
2. Model position or key pattern, type, cardinality, optionality, unit, constraints and conditions only when independently supported.
3. Flag partial fragments, joined declarations and missing condition clauses.
4. Preserve exact examples instead of joining whitespace or interpreting examples as definitions.
5. Deduplicate overlapping source evidence without discarding independent requirements.
6. Compare with manually reviewed reference subsets from **multiple independent protocols**; report both precision and recall.
7. Keep source-conversion quality separate from LLM interpretation quality.

## Next step

Build a small, manually verified, protocol-neutral benchmark with a mixture of tables, prose-only definitions, conditional fields, examples and malformed conversions. Do not introduce format-specific heuristics in the production extractor.
