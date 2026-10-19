# Chunked Transform — Minimal Evidence-Grounded Audit Edit

Review ONE existing specification against ONE supplied evidence excerpt.

Return ONLY one valid JSON object with exactly three string properties:

```json
{
  "evidence_quote": "a verbatim source quote supporting this correction",
  "old_text": "an exact contiguous substring of the existing specification",
  "new_text": "the corrected replacement substring"
}
```

The quote must be at least 12 characters and occur verbatim in the evidence.
The old text must occur exactly once in the specification. Replace only the
smallest necessary passage, preserving everything else. Never return a whole
rewritten file. Do not include Markdown fences, explanations, or FILE records.

Only propose an edit if the evidence proves a concrete correction, omission,
contradiction, or unjustified helper. The new text must be directly supported
by the quote and its immediate context. A partial encoded payload, image,
isolated identifier, or truncated fragment is insufficient. Do not invent
rules, fields, helpers, or source references.

For infrastructure.md, remove unused helpers only if the supplied evidence
establishes they are unnecessary; absence from a single message is not proof.
For general.md, do not rewrite unrelated protocol rules. The separate decision
step should select UNCHANGED when no defensible edit exists.
