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

## Recorded local comparison (October 2026)

The most recent reported `just runtest --all --runs 4` run tested four
configured models across eight fixtures, four times each (128 attempts):

| Provider | Model | PASS | FAIL |
| --- | --- | ---: | ---: |
| `qwen` | Qwen 2.5 Coder 14B, Q5_K_M | 32 | 0 |
| `devs` | Devstral Small 2 24B, Q4_K_XL | 32 | 0 |
| `qwen38q3` | Qwen3.8 27B, Q3_K_XL | 0 | 32 |
| `qwen38q4` | Qwen3.8 27B, IQ4_XS | 0 | 32 |

Qwen 2.5 Coder and Devstral passed every fixture in this run. Devstral's
fix times varied considerably (one Meson factory-refactor attempt took
244.67 seconds). Qwen3.8 Q3 returned HTTP 500 from the model endpoint,
rather than producing a usable repair. Q4 also failed every attempt;
its server behavior requires separate investigation. Neither Qwen3.8
result establishes the model's underlying code-repair capability.

These are **user-run local tests**, not CI results or reproducible performance
benchmarks. A PASS currently means that the fix and build commands returned
success; the runner does not independently verify all API invariants.
There is also a known limitation: some fixture `just fix` recipes return
exit code 0 despite an `aifix` request failure, so the subsequent build
runs against still-broken code. This needs correction before treating
`fix_exit` as a reliable indication of successful model interaction.

### Fix-time statistics by fixture

Measured `just fix` command duration in seconds (s), across four attempts per fixture. Values are rounded from the terminal log to 0.01 s; the generated JSON snapshots retain full precision. These times are **not** end-to-end server startup times. Qwen3.8 times are durations until the unsuccessful request returned, **not successful repair times**.

#### Qwen 2.5 Coder (`qwen`, 32/32 PASS)

| Fixture | Min (s) | Max (s) | Mean (s) | Median (s) |
| --- | ---: | ---: | ---: | ---: |
| cargo/aitestloop_refactor_factory | 20.50 | 61.52 | 37.02 | 33.02 |
| cargo/aitestloop_simple | 3.18 | 4.17 | 3.76 | 3.84 |
| cmake/aitestloop_refactor_factory | 13.63 | 50.56 | 23.13 | 14.16 |
| cmake/aitestloop_simple | 9.18 | 13.99 | 10.40 | 9.21 |
| maven/aitestloop_refactor_factory | 35.15 | 89.58 | 59.80 | 57.23 |
| maven/aitestloop_simple | 19.11 | 20.78 | 19.77 | 19.60 |
| meson/aitestloop_refactor_factory | 9.85 | 24.74 | 16.00 | 14.71 |
| meson/aitestloop_simple | 6.03 | 15.50 | 9.93 | 9.09 |

#### Devstral Small 2 (`devs`, 32/32 PASS)

| Fixture | Min (s) | Max (s) | Mean (s) | Median (s) |
| --- | ---: | ---: | ---: | ---: |
| cargo/aitestloop_refactor_factory | 36.61 | 76.44 | 51.83 | 47.14 |
| cargo/aitestloop_simple | 8.03 | 11.71 | 9.21 | 8.54 |
| cmake/aitestloop_refactor_factory | 23.67 | 33.18 | 27.98 | 27.55 |
| cmake/aitestloop_simple | 9.85 | 49.29 | 22.52 | 15.46 |
| maven/aitestloop_refactor_factory | 27.95 | 65.36 | 39.98 | 33.31 |
| maven/aitestloop_simple | 15.33 | 40.86 | 21.84 | 15.57 |
| meson/aitestloop_refactor_factory | 17.81 | 244.67 | 80.22 | 29.20 |
| meson/aitestloop_simple | 9.68 | 51.36 | 28.98 | 27.44 |

#### Qwen3.8 Q3 (`qwen38q3`, 0/32 FAIL)

| Fixture | Min (s) | Max (s) | Mean (s) | Median (s) |
| --- | ---: | ---: | ---: | ---: |
| cargo/aitestloop_refactor_factory | 13.84 | 14.26 | 13.98 | 13.90 |
| cargo/aitestloop_simple | 13.88 | 14.20 | 13.99 | 13.95 |
| cmake/aitestloop_refactor_factory | 13.76 | 14.41 | 14.07 | 14.06 |
| cmake/aitestloop_simple | 14.14 | 14.67 | 14.34 | 14.27 |
| maven/aitestloop_refactor_factory | 15.16 | 16.06 | 15.43 | 15.25 |
| maven/aitestloop_simple | 15.34 | 15.56 | 15.44 | 15.43 |
| meson/aitestloop_refactor_factory | 13.98 | 14.79 | 14.18 | 13.98 |
| meson/aitestloop_simple | 14.24 | 14.79 | 14.38 | 14.25 |

#### Qwen3.8 Q4 (`qwen38q4`, 0/32 FAIL)

| Fixture | Min (s) | Max (s) | Mean (s) | Median (s) |
| --- | ---: | ---: | ---: | ---: |
| cargo/aitestloop_refactor_factory | 4.80 | 9.90 | 8.05 | 8.76 |
| cargo/aitestloop_simple | 1.57 | 20.73 | 10.82 | 10.50 |
| cmake/aitestloop_refactor_factory | 4.28 | 13.57 | 10.57 | 12.21 |
| cmake/aitestloop_simple | 9.87 | 13.18 | 11.66 | 11.79 |
| maven/aitestloop_refactor_factory | 5.07 | 28.21 | 14.25 | 11.85 |
| maven/aitestloop_simple | 6.03 | 9.43 | 7.97 | 8.21 |
| meson/aitestloop_refactor_factory | 2.48 | 22.26 | 12.04 | 11.71 |
| meson/aitestloop_simple | 5.06 | 12.68 | 8.51 | 8.14 |

Build/check times are separately recorded per attempt in each provider's `results.json`. The values above summarize the **fix phase only**, including failed model requests.

## Automated end-to-end runner

The standalone `aifix_runtest` binary discovers fixture directories with
a `Justfile` under `runtests/{cargo,cmake,meson,maven}/` and runs them
sequentially. For each attempt it invokes `just fix`, then `just build`
when the fix command exits successfully. It records exit codes, timings,
full logs and PASS/FAIL results.

From the repository root:

```sh
just runtest --model devs --runs 4
just runtest --all --runs 4
# Short form: just runtest -a --runs 4
# Optional: just runtest --model qwen --runs 2 --config /path/to/config.json
```

`--all` / `-a` selects providers with `"active": true` in configuration
order; it cannot be combined with `--model`. Each provider gets its own
server startup and shutdown, and failed providers do not prevent the remaining
providers from being tested. The overall command exits unsuccessfully if
any provider fails.

With `--model`, the runner launches `aifix -r` for that provider, waits
for the local health endpoint, and supplies `-p PROVIDER` to fixture calls.
It refuses an already occupied endpoint rather than assuming that the
existing server is the intended model. Server startup retries are bounded;
the runner stops a provider early if its model server exits during testing.
Without `--model`, it uses the configured default provider and expects
the server to be running already.

### Report layout

After each **completed provider run**, the runner writes:

```text
runtests/results/
  qwen/
    results.json
    results.md
  devs/
    results.json
    results.md
  qwen38q3/
    results.json
    results.md
  qwen38q4/
    results.json
    results.md

target/runtest-reports/
  <provider>/
    results.json
    results.md
    model-server.log
    <fixture>_run<N>.log
```

The first tree contains compact, versionable snapshots. The second tree
contains full diagnostic logs and is normally ignored by Git. Direct
`aifix_runtest` execution and `just runtest` both publish snapshots;
`--report-dir PATH` changes the diagnostic report location but not the
snapshot directory. Single-provider runs publish under their provider name
as well. Reports are overwritten for the same provider on subsequent runs.
A provider that terminates before report generation may leave an older
snapshot in place; check its timestamp before relying on it.

`--runs N` repeats each fixture N times. Results contain per-attempt
fix/build exit codes and times, plus min/max/mean/median statistics.
Failed attempts count toward the pass rate and timing statistics. The
JSON also records provider/model settings (excluding API keys).
The runner restores tracked fixture files after each attempt.

**Fixture-local reset:** The fixture's `just fix` recipe restores tracked
files in its directory before testing. The runner restores them again after
each attempt. This can discard uncommitted tracked changes in fixtures;
untracked files remain. The runner currently executes against tracked
fixture directories, not isolated copies.

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
