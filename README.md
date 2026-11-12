# aiagents

> **Experimental Rust project under active development.** The runtime is usable for local experiments, but generated changes and extracted specifications require independent validation.

A local-first agent runtime for AI-assisted software engineering. Rust orchestrates controlled tools and deterministic build/test checks; LLMs handle constrained interpretation and proposed edits. This is **not a security sandbox**.

## Start here

- [Quickstart](QUICKSTART.md) — installation and first run
- [Configuration](docs/README_CONFIG.md) — settings and file-access rules
- [Tasks](docs/tasks/README.md) — one README per task, including maturity and scope
- [Architecture diagrams](docs/architecture_diagrams.md) — workflow diagrams separated by topic
- [Runtime repair / BLT](docs/blt_execute.md) — build, lint, test and repair
- [Chunked extraction](docs/chunked_transform.md) — protocol facts, provenance and limitations
- [Extraction quality criteria](docs/extraction-quality-audit.md) — protocol-neutral acceptance tests and failure modes
- [Runtime tests](runtests/README.md) — fixtures and local results
- [TODO](TODO.md) — pending work
- [Current status and metrics](docs/current_status.md) — dated checkpoint numbers, readiness and pipeline graphs
- [Detailed status and historical results](docs/project_status_history.md) — experimental metrics and design history

## Example

```sh
aifix -l rust -t fix_code -m "Keep the constructor private and update the tests." -f "$(pwd)"
```

Use `-m` for additional literal task instructions; place it before `-a/--args`, which consumes remaining arguments.

## Task catalog

| Task | Purpose |
| --- | --- |
| [`analyze`](docs/tasks/analyze/README.md) | Analysis |
| [`build`](docs/tasks/build/README.md) | Build |
| [`extract_message_tables`](docs/tasks/extract_message_tables/README.md) | Protocol fact extraction |
| [`fix_code`](docs/tasks/fix_code/README.md) | Code repair |
| [`gen_code`](docs/tasks/gen_code/README.md) | Code generation |
| [`release_doc`](docs/tasks/release_doc/README.md) | Release notes, technical review and optional validation/revision |
| [`review_code`](docs/tasks/review_code/README.md) | Code review |
| [`review_doc`](docs/tasks/review_doc/README.md) | Document review |
| [`setup_build`](docs/tasks/setup_build/README.md) | Build setup |
| [`transpile_code`](docs/tasks/transpile_code/README.md) | Project translation |
| [`write_block_doc`](docs/tasks/write_block_doc/README.md) | Type/block documentation |
| [`write_item_doc`](docs/tasks/write_item_doc/README.md) | Item documentation |
| [`write_module_doc`](docs/tasks/write_module_doc/README.md) | Module documentation |
| [`write_test_code`](docs/tasks/write_test_code/README.md) | Test generation |

## Experimental protocol extraction

`extract_message_tables` stages source-attributed facts from overlapping document windows. The current P4 implementation keeps unresolved facts and separates uppercase/lowercase protocol identities. It does **not** guarantee semantic correctness or completeness.

To inspect a **completed** extraction checkpoint without another model call, checkpoint write or publication:

```sh
AIFIX_IDENTITY_REPLAY=1 aifix -t extract_message_tables -a document.md
```

The replay is diagnostic only. Its identity resolver still depends on local source-window signals; generic document-wide definition ownership remains work in progress. The [current status snapshot](docs/current_status.md) records the latest supplied checkpoint metrics with explicit denominators and limitations. The workflow also emits evidence-coverage, semantic-review, conservative codegen-readiness, and a **language-neutral diagnostic protocol model** (`coverage-audit.json.protocol-model.json`). That model retains lexical assemblies and voted table continuations separately from verified protocol definitions; it does not establish canonical message owners or a code-generation-ready schema. See [chunked extraction](docs/chunked_transform.md) and the [task guide](docs/tasks/extract_message_tables/README.md) for the boundaries of these reports. Historical measurements belong in [project status history](docs/project_status_history.md).

## Next extraction milestone

First verify the current branch with a fresh build and test run. Then explain the missing exact statement locators without confusing normalized statements with source quotations. Complete deterministic scoped-context transitions and validate a source-grounded, language-independent schema against **multiple independent documents**. Code generation remains a separate, gated project. See [current status](docs/current_status.md) and [TODO](TODO.md).

## Design and safety

- Prefer deterministic algorithms over LLM calls where practical.
- Treat verbatim source quotations as evidence of text presence, **not** semantic proof.
- File operations use configured access restrictions, but invoked build tools may execute code. Use a container or VM for untrusted projects.
- The project focuses on inspectable workflows and smaller locally hosted models rather than autonomous unsupervised changes.
- Test files live under `tests/`, mirroring the source-module directory hierarchy. See **Test layout** below.

## Test layout

Keep Rust tests separate from production source files while preserving the `src/` → `tests/` module mapping:

- `src/workflows/source_tree.rs` → `tests/workflows/source_tree_test.rs`
- `src/workflows/message_identity.rs` → `tests/workflows/message_identity_test.rs` (plus focused `message_identity_*_tests.rs` modules)
- `src/agenttools/list_dir.rs` → `tests/agenttools/list_dir_test.rs`

**Target layout:** test code belongs exclusively under `tests/` and is registered by Cargo integration-test entry points (such as `tests/mod.rs`) and their normal `mod` declarations. Production modules under `src/` must not import tests via `#[path = "../../tests/..."]`, `include!`, or equivalent test-only source inclusion. The generated production `include!` declarations in `src/lib.rs` are outside this rule.

**Migration in progress:** several existing tests still rely on private module access through `#[cfg(test)] #[path = ...]` declarations in production files. Those tests must be redesigned around supported public behavior, or their underlying functionality refactored behind a deliberate public API, **before** removing the old declarations. Do not silently drop coverage or expose internals solely to satisfy test imports. Once migrated, run `cargo test --all-targets` and `cargo fmt --check`, and verify no source-to-test inclusions remain.

## Dependencies

- [struct_extractors](https://github.com/witold-k/struct_extractors)
- [fsscanner](https://github.com/witold-k/fsscanner)
- Planned underlying component: [TokenDB](https://github.com/witold-k/token_db)

## License

Apache-2.0 © 2026 Witold Kaminski. See [LICENSE](LICENSE).
