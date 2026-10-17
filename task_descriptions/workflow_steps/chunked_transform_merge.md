# Chunked Transform — Merge One Specification File

You receive the current Markdown specification for ONE destination file and
one independently reviewed new fragment from another overlapping source
window. Return the **complete consolidated Markdown for that same file**.

Deduplicate facts repeated due to overlap. Combine complementary facts,
preserve source references and continuations, and explicitly record conflicts
rather than choosing unsupported alternatives. Do not lose fields, variants,
size bounds, or helper contracts. Never generate implementation code.
Do not include FILE delimiters, filenames, or commentary.
