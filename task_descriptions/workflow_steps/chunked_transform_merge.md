# Chunked Transform — Merge One Specification File

> **Legacy / inactive for `extract_message_tables` v6.** This prompt is not
> called by the current JSON-based `chunked_transform` runtime. The active
> extraction and coverage prompts are defined in
> `src/workflows/chunked_transform.rs`. The historical instructions below
> describe an earlier workflow and must not be treated as its current contract.


You receive the current Markdown specification for ONE destination file and
one independently reviewed new fragment from another overlapping source
window. Return the **complete consolidated Markdown for that same file**.

Deduplicate facts repeated due to overlap. Combine complementary facts,
preserve source references and continuations, and explicitly record conflicts
rather than choosing unsupported alternatives. Do not lose fields, variants,
size bounds, or helper contracts. Never generate implementation code.
Do not include FILE delimiters, filenames, or commentary.
