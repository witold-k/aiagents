# Chunked Transform — Independent Source Review

Compare every extracted file fragment against the provided original excerpt.
Return the **complete corrected set of file records**, in exactly the
task-level FILE / END FILE exchange format.

Check for missing messages, fields, shared rules, helpers, and continuations;
verify offsets, sizes, units, constraints, checksums, encodings, source
references, and memory claims. Remove unsupported assertions, preserve
contradictions, and mark unknowns explicitly. Do not generate implementations.
If the source has no relevant facts, return exactly NO_MESSAGE_DEFINITIONS.
