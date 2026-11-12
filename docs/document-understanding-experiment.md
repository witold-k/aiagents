# Experiment: semantic document tree and adaptive windows

Branch: `experiment/document-understanding`

## Implemented first slice

- A source-spanned, parent-linked `SemanticDocumentTree` with scoped context.
- A deterministic `WindowBudget` and `plan()` function, which prefers semantic boundaries, keeps UTF-8 intact, and splits oversized sections without losing their prefix.
- Source span validation and independent integration tests.

This is a **prototype, not yet wired into** `extract_message_tables`. Existing extraction, checkpoint formats, 8,000-character windows, and prompts are unchanged. The prefix is labeled non-evidence. Context entries must not be treated as verified solely because a model proposed them.

## Next milestones

1. Add a read-only **first pass** that builds a source-backed section/block tree from the existing `SourceTree` and `simplelexer` data; attach LLM-suggested semantic labels separately as unverified hypotheses.
2. Integrate the configured model's actual context limit and tokenizer. The current `chars_per_token` estimate is deliberately provisional. Reserve tokens for prompts, the dynamically sized prefix, and output; reject oversized requests before sending them.
3. Handle semantic transitions *inside* a window and produce a source-to-normalized-offset map. Only then wire adaptive planning into an opt-in extraction path.
4. Run identical source documents through old and new planners with the same local model, record definition-level precision/recall, runtime, and token consumption.
5. Keep checkpoints and published JSON versioned; do not silently resume old checkpoints with changed window boundaries.

## Safety constraints

- Never promote a model-suggested heading, owner, or relation to a verified source declaration.
- Preserve source byte spans, exact evidence quotations, and unknown ownership.
- Never drop or duplicate source content at window boundaries.
- Do not introduce protocol-specific heuristics into the general planner.

## Opt-in prepass diagnostic

Run `AIFIX_DOCUMENT_PLAN=1 aifix -t extract_message_tables -a document.md` to build and print a complete-document, deterministic section tree and an experimental window plan **before** the existing extraction. Set `AIFIX_DOCUMENT_CONTEXT_TOKENS` to change the provisional context size (default 32768). The plan uses a conservative character-per-token estimate, **not a model tokenizer**, and does not yet reserve the actual serialized prefix size. Treat it as instrumentation only.

This diagnostic **does not replace** the existing 8,000-character chunks, evidence segmentation, recovery, checkpoint identity, or LLM prompts. The existing extraction proceeds as before. No additional LLM call is made by the prepass. Next, measure actual token usage, derive safe budgets from the configured model, and migrate source-window/checkpoint handling before activating the new planner.

## Opt-in scoped prefix in real LLM requests

`AIFIX_SCOPED_PREFIX=1 aifix -t extract_message_tables -a document.md` enables the deterministic section ancestry prefix for **extraction and coverage LLM requests**. The prefix is derived from source positions, never prior LLM responses. It is read-only context, not evidence. For recovery windows whose source occurrence is ambiguous, no scoped prefix is attached.

This mode deliberately retains the original 8,000-character chunk layout and existing evidence offsets. It changes the extraction fingerprint, so a checkpoint from the legacy mode cannot be resumed as if prompts were identical. The opt-in diagnostic `AIFIX_DOCUMENT_PLAN` remains separate; it does not yet activate adaptive window boundaries.

**Limitations:** The prefix currently describes Markdown heading ancestry, not inferred semantic ownership or complete applicable constraints. The model context limit is not yet queried from the configured backend. A window may still cross a heading boundary, so the prefix describes the start of the window only; the model must respect source-internal changes. Do not treat this as completed semantic two-pass extraction.

## GPU memory / prompt-size diagnostics

Set `AIFIX_LLM_MEMORY_DIAG=1` for per-request logs immediately before primary extraction and coverage calls. The log reports stage, chunk number, prompt/context/prefix character counts, a rough **non-tokenizer** input-token estimate, requested output-token limit, and GPU used/free/total memory (MiB) from `nvidia-smi` when available. This diagnostic is opt-in and does **not** change server context size, prompts, model layers, or inference settings. It does not yet cover every retry, audit, or auxiliary LLM call. GPU figures are process-external snapshots, not exact per-request allocations, and cannot by themselves distinguish CUDA-graph allocation spikes from KV-cache pressure.

For regression isolation, compare a fresh run with `AIFIX_LLM_MEMORY_DIAG=1` against a run additionally enabling `AIFIX_SCOPED_PREFIX=1`, keeping the server configuration identical. Capture both llama.cpp logs and the aiagents MEMORY DIAG lines.
