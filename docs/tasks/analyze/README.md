# Analysis (`analyze`) — NOT IMPLEMENTED

[Task index](../README.md) · [Project overview](../../../README.md)

## Status

**Not implemented.** `task_descriptions/tasks/analyze.md` is empty. No task-specific analysis contract or coherent analysis workflow exists.

The CLI accepts `aifix -t analyze`, and `src/workflows/select_workflow.rs` routes it to the shared `DocWorkflow`. That workflow runs configured build/lint commands and invokes the LLM; it contains no analysis-specific orchestration. **CLI routing is not task implementation.**

## References

- [Empty task description](../../../task_descriptions/tasks/analyze.md)
- [Workflow selector](../../../src/workflows/select_workflow.rs)
- [Shared documentation workflow](../../../src/workflows/doc.rs)
