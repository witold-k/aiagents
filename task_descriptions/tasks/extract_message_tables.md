# Extract Message Tables — Formal Specification

Transform a potentially very large, unknown-format text document into a
**directory of formal Markdown specifications** for a separate, future
code-generation task. Do not generate code, implementations, or build files.

Input: one `.md` or `.txt` file.
Output: `<input_stem>_messages/` beside the source, containing:

- `README.md`: index of message files, source identity, unresolved issues.
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

## Exchange format for each extraction/review response

Return one or more Markdown file records, each using these exact delimiters:

=== FILE: general.md ===
<Markdown content>
=== END FILE ===

=== FILE: infrastructure.md ===
<Markdown content>
=== END FILE ===

=== FILE: messages/<stable-id>.md ===
<Markdown content>
=== END FILE ===

Only these three path patterns are permitted. The stable ID is a lowercase
ASCII slug (letters, digits, hyphen, underscore), derived from the documented
message identifier/name, including version where needed. Reuse the same ID
for the same message in different chunks. Do not emit README.md; it is generated
from the final message index. Each record must be a self-contained, factual
fragment with source line references and explicit continuation/uncertainty
notes. Return exactly `NO_MESSAGE_DEFINITIONS` only if the excerpt has no
relevant message, shared protocol, or infrastructure facts.

Never emit source code, file-system commands, or conversational commentary.
