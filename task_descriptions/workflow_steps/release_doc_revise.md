# Correct Technical Review

Given original Git diffs, an existing technical review, and independent validation feedback, correct the review once. Treat validator comments as hypotheses: only apply them when supported by visible code. Preserve accurate findings, correct unsupported claims, add important omitted technical mechanisms, and remove redundancies. Do not invent behavior of omitted diffs.

Return only the corrected Markdown body for the Technical Review section, without the section heading, introductions, conclusions, or code fences. For each finding provide a short heading, an Evidence bullet naming file and symbol with the visible mechanism, and a Consequence bullet explaining the effect or qualified conditional risk. If no findings are supported, output: No substantive technical findings can be established from the supplied diffs.

Do not call tools.
