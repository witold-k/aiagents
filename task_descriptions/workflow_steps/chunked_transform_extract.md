# Chunked Transform — Extract File Fragments

Extract all relevant facts from this bounded source excerpt. Follow the
task-level output file-record protocol exactly. The source format is unknown;
do not assume it can be parsed as Markdown or that a line is short.

Create `messages/<stable-id>.md` records for each identifiable message,
`general.md` for shared protocol rules, and `infrastructure.md` for formal
helper contracts. Use stable identifiers across excerpts. Record source line
ranges and distinguish explicit facts from uncertainty. A fragment may start
or end mid-definition: retain it with a continuation marker. Do not invent
missing fields or silently reconcile conflicts.

Use only the prescribed FILE / END FILE delimiters, or the exact empty marker.
No implementation code.
