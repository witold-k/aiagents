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

Many chunks contain no relevant facts, including chunks between message
definitions. If the source only contains images, base64, metadata, or
unrelated prose, return exactly NO_MESSAGE_DEFINITIONS. Do not create
placeholder general.md records or describe irrelevant content. Never
combine FILE records and NO_MESSAGE_DEFINITIONS.

A message file requires a protocol-defined message identity supported by explicit
source evidence. An encoded image, base64 blob, example payload, or attachment
is not a protocol message by itself. Never create a message named after its
encoding or media type unless the source explicitly defines that as a protocol
message. If no actual protocol facts remain after excluding such data, return
exactly NO_MESSAGE_DEFINITIONS.
