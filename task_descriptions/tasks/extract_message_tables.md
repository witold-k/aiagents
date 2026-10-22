# Extract Message Tables — Formal Specification

Transform a potentially very large, unknown-format text document into a
**directory of source-grounded JSON facts and Markdown specifications** for a separate, future
code-generation task. Do not generate code, implementations, or build files.

Input: one `.md` or `.txt` file.
Output: `<input_stem>_messages-json/` beside the source, containing:

- `README.md`: generated index of message files and unresolved issues.
- `general.md`: shared protocol rules, framing, encodings, global constraints.
- `infrastructure.md`: formal contracts for required helpers (e.g. bounded
  string parser, byte reader, checksum validator, buffer management).
- `messages/<stable-id>.md`: exactly one specification per logical
  message/version, even if described in several source chunks.

Every relevant message definition, table, or prose description must be
represented. For each message record identifiers, direction, framing, exact
field names/order, byte/bit offsets, widths, types, signedness, encoding,
endianness, units, defaults, reserved bits, variants, constraints, validation,
checksums, size bounds, dependencies, and source references where documented.
Do not invent absent information; preserve conflicts and unknowns.

Analyze fixed/stack/bounded memory suitability and justify any unavoidable
dynamic allocation. Infrastructure components must have responsibilities,
input/output contracts, preconditions, bounds, failure behavior, and
dependencies — **specification only**.

The source format is unknown. Do not assume Markdown syntax, table delimiters,
headings, or parseable embedded media. Source is processed in overlapping,
bounded Unicode character windows. A definition can span windows or occur in
distant places. Deduplicate overlap and consolidate fragments by stable message
identity without discarding contradictory evidence.

## Exchange format for extraction and coverage

Return exactly one JSON object with a `files` array. Each file contains a
safe destination `path` and an array of atomic `facts`. Each fact contains a
`statement` and a verbatim `source_anchor` copied from the current source
window. For example:

```json
{"files":[{"path":"messages/status.md","facts":[{"statement":"Status is transmitted periodically.","source_anchor":"Status is transmitted periodically."}]}]}
```

Allowed extraction destinations are `general.md`, `infrastructure.md`, and
`messages/<lowercase-slug>.md`. Coverage discovers message identities only
and may emit only `messages/<lowercase-slug>.md`. For an empty result return
`{"files":[]}`. Do not return Markdown file delimiters, empty markers, or
prose around the JSON. Never generate `README.md`; the workflow builds the
index deterministically.

Use only source-supported identities and statements. An anchor must be a
contiguous exact substring of the supplied window, at least 12 characters
long and occurring only once there. Rust checks the anchor, normalizes safe
paths, deduplicates exact facts and validates explicit identity conflicts.
These checks are deterministic; do not attempt to emulate them with prose.
Do not infer a protocol message from encoded payloads, illustrations, or
incidental mentions.

## Post-assembly verification

The assembled knowledge must undergo source-grounded checks against
the original source excerpts and other generated specifications. Detect both
omissions and unsupported inventions. Pay special attention to
`infrastructure.md`: each generic helper must be needed by a concrete
message or documented global rule. Identify missing necessary helpers and
remove unjustified abstractions. Flag unresolved evidence explicitly rather than inventing corrections. Do not claim successful
verification if the audit fails to converge.

Never emit source code, file-system commands, or conversational commentary.
