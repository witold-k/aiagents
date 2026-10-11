# Runtime tests

The `runtests/` directory contains deliberately broken, standalone projects
used for **manual end-to-end experiments** with `aifix`. Unlike the Rust
integration tests under `tests/`, these fixtures exercise the full
build/lint/test and LLM repair workflow. They are not part of the root
`cargo test` suite and require a configured LLM service to run `aifix`.

## Test categories

| Scenario | Cargo / Rust | CMake / C++ | Meson / C++ | Maven / Java |
| --- | --- | --- | --- | --- |
| `aitestloop_simple` | Yes | Yes | Yes | Yes |
| `aitestloop_refactor_factory` | Yes | Yes | Yes | Yes |

### `aitestloop_simple` — local compilation error

A minimal Hello World application contains a deliberately invalid expression
or identifier. The intended fix is a small local source correction. This
checks basic build-system detection, diagnostic handling and the code-fix loop.

### `aitestloop_refactor_factory` — API refactoring

The production API has **already been refactored**:

- `Dummy` has a private constructor and a `value()` method returning `1`.
- The factory is the only intended construction path.
- `create(allowed)` returns a value if `allowed` is true, and an empty
  result otherwise.
- Rust uses `Option<Dummy>`; C++ uses `std::optional<Dummy>`;
  Java uses `Optional<Dummy>`.

The **outdated test** still calls the private constructor. The intended
repair is to change the test, **not** the production API: obtain the instance
through the factory, handle the optional result, assert that `value()`
returns `1`, and cover the unsuccessful creation path.

In Java, the factory is a nested `Dummy.Factory` class. The Java test is a
plain class in `src/test/java`; Maven's test-compilation phase detects its
invalid constructor call without a test framework.

## Verified local model

All currently documented runtime fixtures — both `aitestloop_simple` and
`aitestloop_refactor_factory` across Cargo/Rust, CMake/C++, Meson/C++, and
Maven/Java (eight fixture projects) — have been reported working with a
**locally configured Devstral model** (manual end-to-end runs, October 2026).
This is a user-reported validation, not a CI result or a guarantee for other
models, model versions, or configurations. The exact Devstral variant and
quantization were not recorded.

## Automated end-to-end runner

The separate `aifix_runtest` binary discovers all fixture directories containing a
`Justfile` under `runtests/{cargo,cmake,meson,maven}/` and runs them
**sequentially**. For each fixture it runs `just fix` (including the fixture's
Git reset), then `just build` if fixing succeeds. It records fix/build exit
codes, durations, full logs, and PASS/FAIL summaries.

From the repository root:

```sh
just runtest --model devs --runs 5
# Optional: just runtest --model qwen38q4 --runs 3 --config /path/to/config.json
```

`--model` selects a configured provider and starts it using `aifix -r`.
The runner waits for the local server health endpoint, then invokes each
fixture with `aifix -p PROVIDER` (and `-c PATH` if provided). It refuses
to start when the endpoint port is already occupied, rather than assuming
the existing server is the selected model. The runner terminates the server
it started after the tests. Without `--model`, the configured default provider
is used and an existing server is expected.

`--runs N` repeats every fixture N times. Each fixture resets tracked files
before every attempt. Per-fixture results include pass counts, individual
fix/build times, and fix-time min/max/mean/median; build-time statistics are
also stored in JSON when a build was attempted. Failed attempts count toward
the pass rate and retain their timing and logs. Fix-time statistics include
all attempts, including failures; build statistics exclude skipped builds.
The full model/provider settings are recorded in JSON (excluding API keys),
while the root README contains a short summary. Results are overwritten on
each run, as before; Git tracks snapshots when committed.

The recorded 8/8 result below predates these additions and does not include
verified model settings or repeated measurements.

Reports are written to `target/runtest-reports/results.json`,
`results.md`, and per-fixture `.log` files. The root `just runtest` recipe
also copies `results.json` to `runtests/results.json` after execution,
while preserving the runner's exit status. Override the location with
`--report-dir PATH`. The runner exits unsuccessfully if any fixture fails.

**Fixture-local reset:** `just fix` restores tracked files only within the
fixture directory. Changes elsewhere in the repository are not reset, and
the runner does not require a globally clean working tree. Uncommitted
tracked changes inside a fixture will be discarded; untracked files remain.

A PASS currently means that the fix and build commands returned success; it
does **not** independently verify semantic requirements such as preservation
of private constructors. This is a first baseline for automated evaluation.

## Recorded automated results

The committed [results.json](results.json) records one local end-to-end run
with a configured Devstral model. The precise model variant and quantization
were not captured. These are measured command times, not a reproducible
benchmark across different hardware or model configurations.

**8/8 passed; 0 failed; total fix + build command time: 227.05 s (3 min 47 s).**

| Build system | Scenario | Fix (s) | Build (s) | Result |
| --- | --- | ---: | ---: | --- |
| Cargo / Rust | Factory refactor | 48.15 | 0.18 | PASS |
| Cargo / Rust | Simple fix | 8.69 | 0.17 | PASS |
| CMake / C++ | Factory refactor | 29.71 | 0.04 | PASS |
| CMake / C++ | Simple fix | 8.49 | 0.36 | PASS |
| Maven / Java | Factory refactor | 39.00 | 0.87 | PASS |
| Maven / Java | Simple fix | 16.79 | 0.99 | PASS |
| Meson / C++ | Factory refactor | 25.69 | 0.45 | PASS |
| Meson / C++ | Simple fix | 46.96 | 0.52 | PASS |

Both `fix` and `build` exited with code 0 for every fixture. This
indicates command success; the runner does not yet independently check
semantic invariants (such as constructor visibility). The JSON file is the
source of truth; this table is a snapshot of the committed run.

Running `just runtest` overwrites `runtests/results.json` with the
latest run, even when some fixtures fail (provided the runner completes and
writes its report). The detailed logs and Markdown report remain in
`target/runtest-reports/`. Review the new results and update this summary
table before committing a new benchmark snapshot.

## Running a fixture

From a fixture directory, for example:

```sh
cd runtests/cargo/aitestloop_refactor_factory
just build  # Expected to fail before repair
just fix    # Invoke aifix to attempt the repair
just build  # Check the result
```

For the factory fixture, additional instructions can explicitly preserve the
intended API:

```sh
aifix -l rust -t fix_code \
  -m "Do not make the constructor public. Update the tests to use the factory and check both optional outcomes." \
  -f "$(pwd)" -f "$(pwd)/.."
```

Each fixture has its own `Justfile` and normally supports `build`, `fix`,
`fixd` (debug output), and `clean`. The original simple fixtures may also
offer `fixws`, `doc`, or `fixfail`.

**Warning:** `just fix` restores tracked changes within the current fixture
only. Keep any work you want to preserve outside the fixture or commit it.

## What a successful run means

A successful repair should make the fixture build and its checks pass,
**without weakening the intended API** (for example, by making a private
constructor public). A build passing alone does not prove the intended
refactoring was preserved; inspect the resulting diff.

The examples are intentionally failing before repair. Results depend on
the selected model and configuration; the local Devstral results above do not
imply validation across other environments.
