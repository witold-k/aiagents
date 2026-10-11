# aiagents

> **Status:** This is an experimental hobby project under active development.
> The runtime is functional and can be used for real experiments; see the
> [Quickstart](QUICKSTART.md). Test coverage is improving but not yet complete.

## aiagents::aifix - experimental agentic runtime

A small, local-first Rust runtime for AI-assisted software engineering.
It combines build/test feedback, constrained tool access and multi-step
LLM workflows.

## Getting started

See the [Quickstart](QUICKSTART.md) for installation, configuration and
running your first `aifix` task.

To provide additional instructions without creating a task-description file,
pass `-m` (or `--message`) alongside `-t`. For example:

```sh
aifix -l rust -t fix_code \
  -m "Keep the Dummy constructor private; update tests to use the factory and handle Option." \
  -f "$(pwd)" -f "$(pwd)/.."
```

`-m` can be repeated. Its text is added to the task context like additional
`-t <file>` arguments, but is taken literally rather than read from a file.
It does not replace the selected task. Put `-m` before `-a/--args`, which
consumes the remaining command-line arguments.

## Runtime tests and results

`aifix` is exercised against eight deliberately broken end-to-end fixtures
covering Cargo/Rust, CMake/C++, Meson/C++, and Maven/Java. The tests include
simple compilation fixes and factory API refactoring.

A recorded run with a locally configured Devstral model passed **8 of 8
fixtures**, with **227.05 seconds** total measured fix and build time.

| Build system | Simple fix | Factory refactor |
| --- | --- | --- |
| Cargo / Rust | PASS (8.69 s fix) | PASS (48.15 s fix) |
| CMake / C++ | PASS (8.49 s fix) | PASS (29.71 s fix) |
| Meson / C++ | PASS (46.96 s fix) | PASS (25.69 s fix) |
| Maven / Java | PASS (16.79 s fix) | PASS (39.00 s fix) |

These are measured fix-command times; build times, per-fixture exit codes,
the full results table, and limitations are documented in
[runtests/README.md](runtests/README.md). The machine-readable snapshot is
available as [runtests/results.json](runtests/results.json). The exact model
variant and quantization were not recorded, and PASS currently means that
fix and build commands succeeded, not that all API invariants were verified.

Run the full suite from the repository root:

```sh
just runtest --model devs --runs 5
```

The runner starts the selected local model with `aifix -r`, passes `-p`
to fixture calls, and records minimum, maximum, mean, and median fix times
across repetitions. The detailed per-attempt measurements and provider
settings are stored in [runtests/results.json](runtests/results.json) after
the next run. Use `--config PATH` for a non-default configuration.

The 8/8 figures above are a **single historical run**, not five repetitions.

The recipe updates `runtests/results.json` after the run, while detailed logs
remain in `target/runtest-reports/`. It resets tracked changes within each
test fixture; see the [runtime-test documentation](runtests/README.md) before
running it.

## Configuration

See the [configuration reference](docs/README_CONFIG.md) for JSON settings,
file-access filters and path placeholders.

## Current state

`aifix` is a working experimental runtime for automated AI-assisted software
engineering. It can execute multi-step workflows that inspect a repository,
run tools, feed results back to an LLM, and continue from the model response.

Implemented workflows include:

* analyze, build, test, and fix code
* generate and review code
* generate and review documentation
* set up builds
* generate release documentation from Git history

The runtime provides controlled file operations for loading, saving, replacing,
listing, and scanning files and directories. File access is restricted by
configured read/write filters and resolved paths are checked to prevent
escaping those boundaries through symlinks.

The Build/Lint/Test repair workflow is deliberately split into constrained
steps. On a failure it selects relevant source context, diagnoses the problem,
creates and critiques candidate repair designs, synthesizes a final plan, and
only then applies edits. Deterministic build/lint/test execution remains the
validation boundary. See [BLT Execute Workflow](docs/blt_execute.md).

Release documentation is also implemented as a multi-step workflow. It derives
context from Git history, by default since the latest tag, analyzes the changes,
and writes generated release notes to `RELEASE_NOTES.md`. Explicit first/last
revisions can be supplied when a different release range is needed.

The project is usable for experiments today, but it is not intended to be a
general-purpose autonomous coding platform or a security sandbox. The current
focus is local execution with smaller models and deliberately simple,
inspectable mechanisms.

## Design goals

* simplicity
* performance
* some security, but deliberately not a full security sandbox
* local-first: intended to work with smaller LLMs running on consumer hardware
* model independence: no dependency on provider-specific tool-calling APIs
* minimal UI: user interaction should be reduced as much as possible
* consistent test layout: tests live under `tests/`, mirror the relative
  `src/` hierarchy, and use the source filename with a `_test.rs` suffix

`aifix` uses a small text-based tool protocol. Smaller models are not always
reliable at producing strict structured output, so malformed actions are
treated as recoverable: errors can be returned to the model and corrected in
a subsequent attempt. The protocol has been successfully used with 14B-class
models, although this does not imply that such models are capable enough for
every coding task.

## Security model

`aifix` provides file-access restrictions, but **is not a security sandbox**.
Generated changes and build commands can execute code in the local environment.
For untrusted repositories or stronger isolation, run it in a container, VM or
other restricted environment. It requires access to a configured LLM endpoint.

### Repository access

File access can be granted using the `-f` command-line switch. The first entry
allows read/write access; all others are read-only.

If a repository is mounted as:

`/repository`

then `.git` is automatically hidden from the execution environment.

```
/repository          RW
/repository/src      RW
/repository/tests    RW
/repository/.git     inaccessible
```

Git operations are therefore exposed only through controlled runtime
functionality rather than by giving generated code direct access to `.git`.

This restricts direct access to Git metadata through the exposed file tools;
it does not protect against commands executed by build tools or other programs.

## Tasks

The current task system includes operations such as:

* `analyze`
* `build`
* `fix_code`
* `gen_code`
* `write_test_code`
* `review_code`
* `review_doc`
* `transpile_code` (task description exists; workflow not yet implemented)
* `write_item_doc`
* `write_module_doc`
* `write_block_doc`
* `setup_build`
* `release_doc`

Task descriptions are kept separate from the Rust implementation and are
converted into generated Rust metadata during the build.

## Before the first public preview

The following capabilities are planned before the first broader presentation
of the project. They are **not yet claimed as implemented or validated**.

### Code generation from Markdown specifications

Use Markdown specifications produced by **Minuru** as input to `aifix` code
generation workflows. Two initial application areas are planned:

* **Message catalogs:** generate serialization and deserialization code from
  documented message definitions, including sequences of messages or fields.
  Validate lengths, field constraints, optional values, and error handling
  against the specification.
* **Embedded register access:** generate register and bit-field access code
  from Markdown hardware documentation, including addresses, access modes,
  masks, and other documented constraints.

The intended pipeline is **source documents → Minuru → Markdown → aiagents →
generated code → build and tests**. The Markdown extraction pipeline uses Minuru via `pdf_to_text_wrapper` and
already exists separately; generation and end-to-end verification remain planned.

### Automated runtime-test runner

Add a small test suite that runs all fixtures under `runtests/` and evaluates
their outcomes. It should distinguish expected initial failures from successful
repairs, capture logs, and report results by scenario, build system, and model.
For API-refactoring cases, passing the build alone is insufficient: the suite
should also verify that required API constraints remain intact.

## Direction

Pure autonomous code fixing is not the main objective. Smaller local models
are often more useful when work is split into constrained, inspectable steps.
The following areas are therefore the current focus:

* simple reviewer: create review files alongside reviewed source files
* documentation generation, potentially in several steps:

  1. initial agent-generated documentation (existing `write_item_doc`,
     `write_module_doc`, `write_block_doc`)
  2. manual corrections and annotations
  3. post-documentation using an LLM and interactive Q/A with the developer,
     but without direct runtime interaction; communication happens through
     static text files
  4. generated release notes and Git-history summaries, with further
     experiments around useful context and output quality
* automatic tests, or at least initial test stubs
* experiments with local indexing and retrieval for larger source/document
  context

Independent of these goals, automatic code-fix runs are already supported,
but they are not the main objective.

## Dependencies

* [struct_extractors](https://github.com/witold-k/struct_extractors)
* [fsscanner](https://github.com/witold-k/fsscanner)

Future underlying component:

* [TokenDB](https://github.com/witold-k/token_db)

## License

Apache-2.0 © 2026 Witold Kaminski. See [LICENSE](LICENSE).
