# Test generation (`write_test_code`) — NOT IMPLEMENTED

[Task index](../README.md) · [Project overview](../../../README.md)

## Status

**Not implemented.** A task prompt exists, but no coherent test-generation workflow is implemented.

The CLI accepts `aifix -t write_test_code`, and `src/workflows/select_workflow.rs` routes it to the shared `BLTWorkflow`. That workflow runs build/lint/test and, on failure, an LLM-driven **repair** sequence. It does not proactively analyze source files to generate tests, and a successful build/test can terminate without generating any tests.

The prompt in `task_descriptions/tasks/write_test_code.md` describes intended behavior only. Generic routing is not a test-generation implementation.

## References

- [Task prompt](../../../task_descriptions/tasks/write_test_code.md)
- [Workflow selector](../../../src/workflows/select_workflow.rs)
- [Build/lint/test repair workflow](../../../src/workflows/build_lint_test.rs)
