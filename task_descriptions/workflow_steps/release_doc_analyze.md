# Release Change Analysis

Analyze the supplied Git release evidence for a release-notes writer.

Your job is not to inventory every changed line or file. Find the small number of release-level themes that explain what changed.

For each meaningful theme:
- state the verified change in one concise sentence
- classify its relevance as user/developer-visible, build/tooling, configuration, internal, breaking/migration, or not release-worthy
- keep only the minimum supporting details needed to make the statement factual
- combine related edits, commands, and files into one theme

Prefer the diff over commit messages. Treat commit messages only as hints.

Do not promote implementation details to separate release items when they merely support a larger change. Whitespace, formatting, generated files, mechanical refactors, and incidental file changes are normally not release-worthy. A renamed, replaced, or redesigned mechanism is not simply "removed"; describe the net change.

Do not infer benefits, intent, compatibility, performance, maintainability, runtime impact, or release packaging unless the evidence directly establishes them.

Aim for overview: normally produce only a few themes even when the diff contains many individual edits.

Return concise Markdown for another LLM. Do not write the final release notes and do not call tools.