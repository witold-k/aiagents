# Release Change Analysis

Analyze the supplied Git diff as evidence for both release notes and a critical technical review.

Your job is not to inventory every changed line or file. Find the small number of release-level themes that explain what changed.

For each meaningful theme:
- state the verified change in one concise sentence
- classify its relevance as user/developer-visible, build/tooling, configuration, internal, breaking/migration, or not release-worthy
- preserve concrete technical mechanisms: changed control flow, error propagation, fallible calls, dependencies, and tests; explain causal consequences with supporting symbols or diff evidence
- combine related edits, commands, and files into one theme

Prefer the diff over commit messages. Treat commit messages only as hints.

Do not promote implementation details to separate release items when they merely support a larger change. Whitespace, formatting, generated files, mechanical refactors, and incidental file changes are normally not release-worthy. A renamed, replaced, or redesigned mechanism is not simply "removed"; describe the net change.

Do not infer benefits, intent, compatibility, performance, maintainability, runtime impact, or release packaging unless the evidence directly establishes them.

Aim for a readable overview, but retain technically consequential internal changes for the subsequent review. Distinguish verified behavior, plausible inference, and unresolved cross-file interactions. Do not invent risks to fill a checklist.

Return Markdown for another LLM, without wrapping the result in a code fence. Do not write the final release notes and do not call tools.