# Documentation map

[Project overview](../README.md) · [Current status](current_status.md) · [Quickstart](../QUICKSTART.md)

The repository currently contains **67 Markdown files** at the 2026-11-12 documentation scan (excluding generated files outside the Git tree). They serve different purposes; not every Markdown file is a user-facing guide.

| Area | Canonical entry point | Role |
| --- | --- | --- |
| Project overview | [README](../README.md) | What the runtime does, task catalog and safety boundaries |
| Setup | [Quickstart](../QUICKSTART.md) · [Configuration](README_CONFIG.md) | Local installation, LLM service and access settings |
| Current development state | [Current status](current_status.md) · [TODO](../TODO.md) | Dated evidence, limitations, priorities |
| Architecture and diagrams | [Architecture diagrams](architecture_diagrams.md) | Runtime, extraction, identity, diagnostics and verification gates |
| Extraction internals | [Chunked extraction](chunked_transform.md) · [Window context](window_context/README.md) | Implemented pipeline and proposed scoped-context state |
| Extraction quality | [Semantic coverage](semantic-coverage-audit.md) · [Quality criteria](extraction-quality-audit.md) | Diagnostic measures, validation gaps and generic acceptance criteria |
| BLT | [Build/lint/test repair](blt_execute.md) | Repair cycle and deterministic validation |
| Tasks | [Task index](tasks/README.md) | User-facing task maturity and scope |
| Task/prompt definitions | [Task descriptions](../task_descriptions/) · [Task requirements](../task_requirements/) | Runtime prompt/configuration inputs; **not** automatically synchronized with guides |
| Internal maintenance | [Internal update task](../aitask_internal/update_doc.md) | Development instructions |
| Historical context | [Project history](project_status_history.md) · [Identity review](reviews/p4_identity_checkpoint_audit.md) | Older decisions, audits and replay findings |
| Test experiments | [Runtime tests](../runtests/README.md) · `runtests/results/*/results.md` | Local fixture results, not CI guarantees |

## Keeping documentation current

1. Update **current status** from a named source revision and checkpoint, distinguishing replay from fresh extraction.
2. Update the relevant technical guide and its Mermaid graph when data flow, trust boundaries or JSON fields change.
3. Keep task instructions and task READMEs distinct: prompt changes can affect checkpoint fingerprints.
4. Preserve dated experimental results in **history** instead of silently rewriting their original claims.
5. Validate Markdown links and Mermaid rendering before merging; no CI documentation build was executed as part of this update.
6. Avoid protocol-specific assumptions in generic extraction guidance and fixtures.

**Known follow-up:** audit the full set of Markdown cross-links and runtime prompt wording with an automated repository-local check. This index is a navigational classification, not a claim that every prompt or historical note has been semantically revalidated.
