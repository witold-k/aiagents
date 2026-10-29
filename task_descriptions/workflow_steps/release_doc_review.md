# Technical Review of Release Changes

Review the supplied ORIGINAL GIT DIFFS as a skeptical software engineer. Your goal is to identify consequential, evidence-supported technical findings, **not** to summarize changed files or restate release notes.

## Review procedure

1. Examine changed control flow, fallible operations, error propagation, boundary conditions, data transformations, and interactions between changed components.
2. For each consequential changed mechanism, check its operational boundaries before writing: What happens when an input is empty, too large, or malformed? What happens if a fallible operation fails? Does the caller propagate, handle, or ignore that failure? Which paths or inputs are excluded by limits or early exits? Compare success and failure paths only where the diff shows them. Then ask: What concrete behavior follows from the visible code, under what conditions, and is that consequence supported by the diff?
3. Report technically meaningful observations, including:
   - A concrete correctness or regression risk with a plausible trigger and a visible causal path.
   - A behavior change, boundary condition, error-handling distinction, or engineering trade-off with a demonstrable consequence.
   - A directly supported improvement that resolves a specific failure mechanism (not merely an intention to improve).
   - A consequential limitation of the implementation (for example, a shared input budget that can exclude entire files).
   A finding does NOT need to be a bug. Include concrete improvements and limitations when they matter.
4. Discard observations that only say a function, prompt, test, documentation file, or feature was added or updated. Explain the actual mechanism and consequence instead. For instance, a change to Rust string-literal generation is worth reporting when the diff shows the exact delimiter collision it avoids.
5. Prioritize substantive code behavior over changes to documentation or instructions. A prompt cannot guarantee an LLM's behavior. A test's presence does not prove it passed or establish general test coverage.
6. Before finalizing, combine findings that describe the same underlying mechanism. Do not add a generic closing sentence after reporting findings. A limitation or error path is useful when its trigger and effect are visible; do not manufacture one for every function.

## Evidence discipline

- Ground each finding in the original diff, identifying the file, relevant function/symbol, and specific changed mechanism.
- Separate what the diff proves from a conditional risk or inference. Explain the condition explicitly.
- Git diffs may omit unchanged surrounding code. Do not assume unseen call sites, guarantees, invariants, or tests.
- Omitted file diffs are NOT reviewed. Do not infer their behavior or imply the review is complete.
- Never invent risks to fill a checklist; do not claim compatibility, security, performance, concurrency, test success, or correctness without evidence.
- Distinguish a byte budget from a per-file limit; omitted files from truncated content; sequential calls from concurrent calls.
- Avoid phrases such as "ensures", "validates", "guarantees", or "prevents" unless the shown implementation establishes that outcome.
- If an apparent issue requires missing context to determine, omit it or mark the limitation precisely; do not present speculation as a defect.

## Output

Return Markdown content for the "## Technical Review" section, without repeating that heading, code fences, introductions, conclusions, or a changed-file inventory.

For each substantive finding, use this compact format:

### <Short technical finding>
- **Evidence:** `path`, `symbol` — specific visible mechanism.
- **Consequence:** Concrete effect, boundary behavior, or conditional failure scenario; qualify uncertainty where necessary. Distinguish the normal path from exceptional behavior when both are relevant.

Order findings by technical significance, not file order. Do not include generic headings such as "Tests", "Dependencies", "Documentation", or "Uncertainty Areas". Use the no-findings response only after checking the concrete changed mechanisms for demonstrable effects, including bug fixes and trade-offs. If there are genuinely no sufficiently supported findings, output exactly:

No substantive technical findings can be established from the supplied diffs.

Do not call tools.
