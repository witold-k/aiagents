Here is a concise specification you can use as the rationale/design note for `done` and `failed`, incorporating your requirements and the conclusions we reached.

# Design Requirements for `done` and `failed`

## 1. Purpose

The `done` and `failed` actions control whether the agent finishes the current task or declares that it cannot continue.

They must encourage the model to **actively solve the task**, while avoiding unnecessary hesitation, premature failure, or endless attempts.

The model should focus on what it can determine from the **current available information and tool results**.

## 2. Agent Should Commit Changes

The agent must not be afraid to commit a reasonable change.

When a task requires a modification:

* The agent should make the change rather than merely describe it.
* The agent does not need absolute certainty that the change is perfect.
* A reasonable, task-directed solution should be committed when the available information supports it.
* An imperfect change is acceptable when external verification can detect and report the problem.

This applies to all file types, not only source code.

For example:

* Code may be checked by a compiler, linter, or tests.
* Documentation may be checked by a formatter, validator, or later review.
* Configuration may be checked by a parser or deployment validation.
* Other artifacts may have their own post-processing or validation.

The agent's role is to **make the best reasonable change**, not to prove independently that the result is perfect.

## 3. External Verification

Verification is primarily performed outside the model.

The agent may receive results from external tools such as:

* compilers
* linters
* tests
* formatters
* validators
* document processors
* other post-processing systems

When external verification reports a problem, the agent should use that information to diagnose the problem and attempt a concrete correction.

The agent should not avoid making a change simply because external verification has not yet confirmed it.

## 4. `done` Requirements

The agent should call `done` when the task's completion requirements are satisfied.

If the task explicitly requires external verification:

1. Make the required changes.
2. Allow the external system to perform the required verification.
3. If verification reports an error, attempt a correction.
4. Call `done` when the required verification reports success.

If no external verification is required, call `done` when the requested operation has been completed.

The agent should **not require absolute certainty** before calling `done`.

## 5. `failed` Requirements

`failed` is a last-resort action.

The agent should make a serious effort to solve the task before using it.

The agent should:

* inspect the available information;
* use the available files and tool results;
* consider the task requirements;
* use external verification results when available;
* attempt reasonable corrections;
* consider another reasonable approach when an initial approach fails.

`failed` should NOT be used merely because:

* the first solution did not work;
* a compiler reports an error;
* a linter reports an error;
* tests fail;
* documentation validation fails;
* the model is uncertain whether a change is perfect;
* a reasonable correction can still be attempted.

## 6. When `failed` Is Appropriate

Use `failed` only when there is no reasonable path to continue with the available information and tools.

Examples include:

* the required file cannot be obtained;
* the required context cannot be determined;
* the file state is physically unavailable or unrecoverable;
* required information is missing and cannot be obtained;
* the environment prevents the required operation;
* no reasonable next action can be determined.

The model should not use `failed` simply because it cannot guarantee success.

## 7. Do Not Depend on Invisible History

The specification must not require the model to detect things it cannot reliably observe.

In particular, the model may not have reliable access to:

* previous attempts;
* complete tool history;
* previous file versions;
* whether another agent changed the file;
* whether an earlier change was correct;
* whether the agent is currently in a loop.

Therefore, `done` and `failed` should be based on the **current available state**, not assumptions about invisible history.

## 8. Avoid Artificial Loop Rules

The model should not be instructed to detect or reason about loops such as:

> "I already tried this."

if that history is not reliably available.

Likewise, the specification should not require the model to determine whether a previous change was "correct" when that information may no longer exist.

Loop detection and execution limits are better handled by the external agent runner when possible.

The model's responsibility is simpler:

> Determine whether there is a reasonable action available now. If there is, take it. If there is not, and the task cannot reasonably be completed, use `failed`.

## 9. Core Principle

The intended behavior is:

**Be persistent, but not paralyzed by uncertainty.**

The agent should:

1. Understand the current task and available information.
2. Make a reasonable change when one is possible.
3. Commit the change.
4. Use external verification when available.
5. Use verification feedback to make further corrections.
6. Continue making a serious effort while reasonable solutions remain available.
7. Call `done` when the completion requirements are satisfied.
8. Call `failed` only when no reasonable path to completion remains.

The goal is not for the model to prove that every change is perfect.

The goal is for the model to **actively solve the task and use external verification to catch and guide corrections**.

