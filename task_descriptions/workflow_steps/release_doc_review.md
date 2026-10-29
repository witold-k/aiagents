# Release Technical Review

Produce a conservative, evidence-based technical review of the supplied ORIGINAL GIT DIFFS. This is not a release-notes summary.

Evidence rules:
- Only assert what the included diff lines directly establish, or a clearly qualified consequence of their visible control flow. The diff may omit unchanged surrounding code.
- Each substantive finding must name a file and relevant function/symbol and cite a concrete changed code fragment or mechanism. Explain the causal consequence, not just the change.
- Do not treat commit messages, comments, tests, or instructions as proof of runtime behavior. A test's existence does not prove it passed.
- Do not infer behavior of omitted files or missing unchanged context. Never turn lack of evidence into a claimed defect.
- Omit claims that cannot be substantiated. Zero findings is acceptable. Do not fill sections with generic praise, risk lists, or speculation.
- Do not claim operations are parallel unless concurrency is visible. Do not call omission truncation. Distinguish byte limits from character limits.
- Distinguish fallible calls from infallible string processing; identify the actual caller that handles each error.
- Do not claim backward compatibility, test coverage, safety, performance, or CI behavior without direct evidence.
- Avoid duplicating release notes or providing a catalog of changes. Focus on concrete correctness, error handling, and regression implications where supported.
- If evidence is insufficient, say so briefly.

Return only Markdown beneath the "## Technical Review" heading, without repeating the heading. A short list of precise findings is preferable to a comprehensive report. Do not call tools.
