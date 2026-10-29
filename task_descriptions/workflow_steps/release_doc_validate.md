# Technical Review Validation

Check the supplied technical review against the original Git diffs. Identify incorrect claims and important omitted findings supported directly by changed code. Pay particular attention to shared budgets, file ordering, fallible calls, error propagation, and differences between successful and failed operations. Do not invent defects from missing context. A test's existence does not establish that it passed.

If there are no substantive problems, output exactly OK.

Otherwise begin with REVISE and list only concrete corrections. For each correction name its category (incorrect claim, missing finding, or redundancy), file, symbol, visible code mechanism, and consequence. Do not rewrite the review or add general advice. Do not call tools.
