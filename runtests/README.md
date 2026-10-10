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

**Warning:** `just fix` uses `git checkout -- .` to reset tracked
modifications. It can discard uncommitted changes in the fixture. Use a
clean working tree or a disposable checkout.

## What a successful run means

A successful repair should make the fixture build and its checks pass,
**without weakening the intended API** (for example, by making a private
constructor public). A build passing alone does not prove the intended
refactoring was preserved; inspect the resulting diff.

The examples are intentionally failing before repair. Results depend on
the selected model and configuration, and are not claimed to be validated
across all environments.
