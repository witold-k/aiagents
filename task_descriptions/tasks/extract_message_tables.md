# Extract Message Tables — Requirements

## Objective

Transform a potentially very large Markdown or plain-text protocol description
into a **formal Markdown specification of every message**.

Input: one `.md` or `.txt` file.
Output: `<input_stem>_messages.md` beside the input.

**Do not generate executable code, source files, or implementations.**
Specify required helper types/classes and their interfaces in prose and tables.

## Scope

- Extract every message definition, including message identifiers, direction,
  purpose, framing, headers, fields, payloads, and nested structures.
- Preserve exact field names, offsets, bit positions, widths, units, signedness,
  encodings, endianness, alignment, padding, reserved values, and defaults when
  explicitly documented.
- Record discriminators, version differences, optional/conditional fields,
  variable-length arrays, repetition counts, enumerations, and constraints.
- Include validation rules, checksums/CRCs, escaping, delimiters, length
  calculations, error handling, and parsing order when supported by the source.
- Specify parsing/serialization helper abstractions (e.g. byte reader,
  bounded string parser, checksum calculator) and their contracts **without code**.
- Analyze memory needs: fixed-size versus bounded variable-size buffers,
  worst-case sizes, stack/static suitability, and whether dynamic allocation is
  *actually* necessary. Prefer no dynamic allocation; justify exceptions.
- Identify dependencies between messages and definitions that may appear in
  other sections. Preserve cross-references and source line ranges.
- Never invent undocumented fields, limits, protocols, or implementation
  guarantees. Mark missing or contradictory information as unresolved.

## Per-message format

### Message: <documented name / identifier>

**Purpose / direction / framing:** only supported facts.

| Offset / bit | Field | Type / width | Encoding / endian | Constraints / meaning | Source |
| --- | --- | --- | --- | --- | --- |

**Size and allocation:** minimum/maximum size, bounds, buffering strategy,
and justification for any dynamic allocation.

**Parsing and validation:** ordering, checks, error conditions, dependencies.

**Required helpers (specification only):** name, responsibility, input/output
contracts, preconditions, bounds, error behavior; no code.

**Open questions:** unsupported assumptions, ambiguities, conflicts.

## Quality requirements

1. Account for every relevant source table and message, including prose-only
   descriptions; do not confuse unrelated tables with message definitions.
2. Preserve source traceability and do not silently reconcile contradictions.
3. Treat references to external specifications as unresolved unless their
   contents are actually present in the supplied document.
4. Use clear, consistent Markdown headings and tables; retain precise units.
5. Never omit a message merely because it is split across source chunks.
6. The workflow must process large documents in bounded chunks and validate
   each extracted section against the corresponding original source excerpt.
7. If a definition spans chunks, retain its continuation and explicitly
   indicate where manual cross-chunk consolidation may be required.
8. The final file contains only formal specifications and unresolved issues,
   not generated source code.
