# Task implementation status

This index describes **runtime wiring**, not proof that a task fulfills every promise in its prompt. Verified against `src/workflows/select_workflow.rs` and the corresponding `execute()` methods on 28 October 2026. All listed task names are selectable, but their implementations differ substantially.

| Task | Runtime implementation | Boundary | Documentation |
| --- | --- | --- | --- |
| `analyze` | **Not implemented** | Empty task prompt; generic `DocWorkflow` routing exists but no meaningful analysis contract or process | [Analysis](analyze/README.md) |
| `build` | `BuildWorkflow` | Runs build command; on failure may invoke LLM; not strictly non-LLM | [Build](build/README.md) |
| `extract_message_tables` | Dedicated `ChunkedTransformWorkflow` | Experimental source-grounded extraction, identity staging, checkpoint and JSON projection | [Protocol extraction](extract_message_tables/README.md) |
| `fix_code` | Dedicated `BLTWorkflow` | Build/lint/test with LLM repair cycle | [Code repair](fix_code/README.md) |
| `gen_code` | **Not implemented** | CLI routing to generic `EndlessWorkflow` exists, but no usable task-specific generation process | [Code generation](gen_code/README.md) |
| `release_doc` | Dedicated `ReleaseDocWorkflow` | Git-range analysis and release-note output | [Release notes](release_doc/README.md) |
| `review_code` | Shared `DocWorkflow` | No dedicated review-code orchestration | [Code review](review_code/README.md) |
| `review_doc` | Shared `DocWorkflow` | No dedicated review-doc orchestration | [Document review](review_doc/README.md) |
| `setup_build` | Dedicated `SetupBuildWorkflow` | Delegates to build-system setup | [Build setup](setup_build/README.md) |
| `transpile_code` | **Not implemented** | CLI routing to generic `EndlessWorkflow` exists, but no usable task-specific translation process | [Project translation](transpile_code/README.md) |
| `write_block_doc` | Shared `DocWorkflow` | No dedicated block-documentation orchestration | [Block documentation](write_block_doc/README.md) |
| `write_item_doc` | Shared `DocWorkflow` | No dedicated item-documentation orchestration | [Item documentation](write_item_doc/README.md) |
| `write_module_doc` | Shared `DocWorkflow` | No dedicated module-documentation orchestration | [Module documentation](write_module_doc/README.md) |
| `write_test_code` | **Not implemented** | CLI routing to repair-oriented `BLTWorkflow` exists, but no usable test-generation process | [Test generation](write_test_code/README.md) |

## How to interpret the status

- **Dedicated:** a task-specific workflow exists, though its correctness still needs testing.
- **Shared:** a generic workflow runs with task-specific prompts; the prompt's goals are not automatically enforced. This does not by itself mean the task is implemented.
- **Not implemented:** a task name, prompt and/or generic CLI routing may exist, but no functional task-specific process is implemented. This applies to `analyze`, `gen_code`, `transpile_code` and `write_test_code`.
- **Prompt-only contract:** task instructions describe desired behavior but are not independent implementation evidence.
- **Experimental:** implemented but explicitly not validated as production-ready.

The authoritative routing is [`select_workflow.rs`](../../src/workflows/select_workflow.rs). Relevant execution modules are [`build.rs`](../../src/workflows/build.rs), [`build_lint_test.rs`](../../src/workflows/build_lint_test.rs), [`doc.rs`](../../src/workflows/doc.rs), [`endless.rs`](../../src/workflows/endless.rs), [`setupbuild.rs`](../../src/workflows/setupbuild.rs), [`release_doc.rs`](../../src/workflows/release_doc.rs) and [`chunked_transform.rs`](../../src/workflows/chunked_transform.rs).

The `task_descriptions/tasks/` directory contains prompt contracts, **not** implementation-status evidence. In particular, changing `extract_message_tables.md` can change the checkpoint fingerprint; coordinate such changes with checkpoint compatibility.

[Project README](../../README.md) · [Architecture](../architecture_diagrams.md)
