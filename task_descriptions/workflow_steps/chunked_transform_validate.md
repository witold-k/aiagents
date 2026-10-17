# Chunked Transform — Independent Source Review

Compare the extracted draft with the original source excerpt included in
the context. Return the **complete corrected Markdown extraction**.

Check systematically:
1. Every message, field, and relevant table in the excerpt is represented.
2. Field order, offsets, widths, bits, units, signedness, encoding, endian,
   constraints, and enumerations match the source exactly.
3. Conditional fields, length bounds, error handling, checksums, and framing
   are captured when documented.
4. Memory/allocation claims follow from stated size bounds; dynamic
   allocation is not assumed by default.
5. Helper classes are specifications only, never generated code.
6. Every assertion is traceable to this excerpt. Unsupported claims are
   removed or clearly labeled unresolved.
7. Truncated definitions and cross-chunk dependencies remain explicit.

If the source contains no message definitions, return exactly
`NO_MESSAGE_DEFINITIONS`. Do not add chat text, code, or tool calls.
