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
- [Runtime tests](runtests/README.md) — fixtures and local results
- [TODO](TODO.md) — pending work
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

The replay is diagnostic only. Its identity resolver still depends on local source-window signals; generic document-wide definition ownership remains work in progress. In a recent OpenAMIP checkpoint replay, **25 candidates** and **238 source-attributed facts** were retained; **78 distinct source-backed parameter assertions** were inventoried. These are checkpoint-specific structural observations, not a semantic correctness score. The workflow now also emits global evidence-coverage, semantic-review, conservative codegen-readiness, and a **language-neutral diagnostic protocol model** (`coverage-audit.json.protocol-model.json`). The latter preserves candidate assertions and source anchors and detects literal `key=value` hints; it does not yet establish canonical messages or field schemas. The recent replay found **14 lexical parameter hints across 10 candidates**, compared with the separate table audit's **78 source-backed parameter assertions**. These counts have different meanings and must not be treated as equivalent coverage metrics. See [chunked extraction](docs/chunked_transform.md) and the [task guide](docs/tasks/extract_message_tables/README.md) for the boundaries of these reports. Historical measurements belong in [project status history](docs/project_status_history.md).

## Next extraction milestone

Reuse existing normalized `simplelexer` table cells, source positions, and proposed semantic continuation links to distinguish parameter **definitions** from examples. Capture type, range, optionality and conditional rules only when individually supported by source evidence; preserve unresolved message ownership. Validate the resulting language-independent model against the technical review gaps before treating it as suitable input for a **separate** C++/Rust generator. This is a targeted model-quality step, not another round of document-administration filtering. See [TODO](TODO.md) for the prioritized checklist.

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

Unit tests that need private module access are included under `#[cfg(test)]` using `#[path = "../../tests/..."]`; these paths refer only to test builds. Public API integration tests remain regular Cargo integration tests. Keep test-only compatibility helpers behind `#[cfg(test)]` where needed, and avoid exposing production APIs solely for tests. Run `cargo test` and `cargo fmt --check` when changing the layout.

## Dependencies

- [struct_extractors](https://github.com/witold-k/struct_extractors)
- [fsscanner](https://github.com/witold-k/fsscanner)
- Planned underlying component: [TokenDB](https://github.com/witold-k/token_db)

## License

Apache-2.0 © 2026 Witold Kaminski. See [LICENSE](LICENSE).
