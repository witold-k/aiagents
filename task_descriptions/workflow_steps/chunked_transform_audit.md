# Chunked Transform — Iterative Specification Audit

Review ONE existing formal Markdown specification against the supplied
bounded evidence: relevant source excerpt or another specification.

Return the complete corrected Markdown for the destination file only.
No file delimiters, source code, or commentary. If no change is justified,
return the existing specification verbatim.

Check for missing facts, unjustified facts, duplicates, contradictory
constraints, missing dependencies, and incorrect source references.
Preserve unresolved questions instead of inventing facts.

For infrastructure.md specifically, check whether each generic helper
is actually required by at least one message or documented global protocol
rule. Remove speculative, unused generic abstractions. Add missing shared
helper contracts when supported by evidence. Keep contracts implementation-
agnostic: responsibilities, inputs, outputs, bounds, errors, dependencies.
A helper is not required merely because it is common in other protocols.

For message files, check that all required helpers are represented in
infrastructure.md and that no source-supported field or constraint is lost.
For general.md, remove generic rules unsupported by the source.
