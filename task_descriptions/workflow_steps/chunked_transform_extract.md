# Chunked Transform — Message Extraction

You are extracting protocol message specifications from ONE excerpt of a much
larger source document. Follow the task requirements supplied in this prompt.

Return **Markdown only**, not code or tool calls.

- Extract all message definitions, tables, and related rules visible in this
  excerpt. A message may be defined in prose rather than a Markdown table.
- Preserve source line references from the excerpt header.
- Retain exact units, field order, sizes, constraints, and conditional logic.
- Specify necessary helper classes/functions only as formal interfaces and
  behavior, never implementations.
- Assess fixed/bounded memory and dynamic allocation requirements. Do not
  assert dynamic allocation is necessary without explicit justification.
- If the excerpt begins or ends in the middle of a definition, mark it as a
  continuation, and preserve all available details for later reconciliation.
- If no relevant message information appears, return exactly
  `NO_MESSAGE_DEFINITIONS`.
- Never infer missing values or resolve conflicts without evidence.
- Do not include generic introductions or summaries.
