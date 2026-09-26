# Code Fix Source Selection

Select the source files that are most useful for diagnosing the supplied compiler, linter, or test failure.

Choose only paths that appear exactly in the supplied FILE LIST.

Return at most two file paths, one path per line. Return only the paths, with no bullets, explanation, Markdown, or code fences.

Prefer files that define the failing type, function, constructor, operator, or API. Do not select files merely because they appear in the diagnostic when another listed file is more likely to contain the required definition.
