# Chunked Transform — Independent Source Review

> **Legacy / inactive for `extract_message_tables` v6.** This prompt is not
> called by the current JSON-based `chunked_transform` runtime. The active
> extraction and coverage prompts are defined in
> `src/workflows/chunked_transform.rs`. The historical instructions below
> describe an earlier workflow and must not be treated as its current contract.


Compare every extracted file fragment against the provided original excerpt.
Return the **complete corrected set of file records**, in exactly the
task-level FILE / END FILE exchange format.

Check for missing messages, fields, shared rules, helpers, and continuations;
verify offsets, sizes, units, constraints, checksums, encodings, source
references, and memory claims. Remove unsupported assertions, preserve
contradictions, and mark unknowns explicitly. Do not generate implementations.
If the source has no relevant facts, return exactly NO_MESSAGE_DEFINITIONS.

A chunk with no message or protocol facts is normal anywhere in the document.
If the draft only describes images, base64, metadata, or unrelated text,
return exactly NO_MESSAGE_DEFINITIONS. Do not emit general.md for such
observations. Never combine FILE records and NO_MESSAGE_DEFINITIONS.

A message file requires a protocol-defined message identity supported by explicit
source evidence. An encoded image, base64 blob, example payload, or attachment
is not a protocol message by itself. Never create a message named after its
encoding or media type unless the source explicitly defines that as a protocol
message. If no actual protocol facts remain after excluding such data, return
exactly NO_MESSAGE_DEFINITIONS.
