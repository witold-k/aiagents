# Architecture diagrams by topic

These diagrams describe responsibilities, not a claim of semantic correctness.
Labels distinguish **LLM** from **deterministic Rust** operations without relying on color.

## 1. Runtime and repair workflow

```mermaid
flowchart TD
    U["User task and configured access"] --> R["Rust: workflow orchestration"]
    R --> L["LLM: propose actions / repair"]
    L --> T["Rust: validate and dispatch controlled tools"]
    T --> B["Build / lint / test commands"]
    B --> R
```

See [BLT execution](blt_execute.md) for the detailed repair cycle and its limitations.

## 2. Chunked document extraction

```mermaid
flowchart TD
    S["Source document"] --> W["Rust: bounded overlapping windows"]
    W --> E["LLM: extract candidate facts"]
    E --> V["Rust: validate evidence and stage candidates"]
    V --> C["Rust: checkpoint"]
    C --> G["LLM: coverage sweep"]
    G --> C
```

## 3. Identity projection and provenance

```mermaid
flowchart TD
    C["Checkpointed candidates and source windows"] --> P["Rust: verify source locators"]
    P --> I["Rust: resolve explicit source type signals"]
    I -->|consistent| M["Canonical case-sensitive message groups"]
    I -->|missing / ambiguous / conflicting| U["Unresolved facts retained"]
    M --> D["Rust: deduplicate occurrences, preserve provenance"]
    D --> A["Fact audit"]
    A --> O["Unverified or reviewed JSON publication"]
```

The current resolver is **window-local** despite the document-wide goal. It does not
yet construct a generic document-wide definition index. An explicit type signal
does not by itself prove that a message is defined rather than reserved or only
mentioned in examples. The diagnostic replay is read-only and stops before
audit/publication.

## 4. Resume and diagnostic replay

```mermaid
flowchart TD
    X["Existing completed checkpoint"] --> Q{"Replay requested?"}
    Q -->|yes| I["Read-only identity projection"]
    I --> D["Diagnostic counters; no write or model call"]
    Q -->|no| P{"Output already published?"}
    P -->|yes| Z["Return without overwrite"]
    P -->|no| A["Normal workflow / audit / publication"]
```

Replay example:

```sh
AIFIX_IDENTITY_REPLAY=1 aifix -t extract_message_tables -a document.md
```

## 5. Verification boundaries

```mermaid
flowchart LR
    Q["Exact source quotation"] --> S["Quote-supported fact"]
    S --> M["Semantic / identity review still required"]
    M --> V["Only independently validated specifications should drive code generation"]
```

A byte-for-byte matching quotation is **not** semantic verification.
For detailed extraction behavior, see [Chunked Transform](chunked_transform.md).
