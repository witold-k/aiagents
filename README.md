# aiagents

> **Status:** This is an experimental hobby project under active development.
> The runtime is functional and can be used for real experiments; see the
> [Quickstart](QUICKSTART.md). Test coverage is improving but not yet complete.

## aiagents::aifix - experimental agentic runtime

A small Rust runtime for AI-assisted software engineering ... or maybe other
tasks.

## License

Apache-2.0 (C) Witold Kaminski 2026

## Quickstart

Please follow the instructions in:
[Quickstart](QUICKSTART.md)

## Current state

`aifix` is a working experimental runtime for automated AI-assisted software
engineering. It can execute multi-step workflows that inspect a repository,
run tools, feed results back to an LLM, and continue from the model response.

Implemented workflows include:

* analyze, build, test, and fix code
* generate and review code
* generate and review documentation
* generate initial test code
* set up builds and transpile code

The runtime provides controlled file operations for loading, saving, replacing,
listing, and scanning files and directories. File access is restricted by
configured read/write filters and resolved paths are checked to prevent
escaping those boundaries through symlinks.

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

### Simplicity

Well, this is a security feature of its own.

### General

Except for file filters that restrict reading and writing arbitrary data,
there are no additional security mechanisms. This is by design.

The main design goal is to keep the runtime simple and small. If additional
isolation is required, execute `aifix` inside Docker, Podman, a VM, or even on
a dedicated isolated PC.

`aifix` is a single binary without additional runtime libraries. It only needs
HTTPS access to an LLM service.

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

This prevents generated code from directly modifying Git metadata, hooks,
refs, or objects.

## Tasks

The current task system includes operations such as:

* `analyze`
* `build`
* `fix_code`
* `gen_code`
* `write_test_code`
* `review_code`
* `review_doc`
* `transpile_code`
* `write_item_doc`
* `write_module_doc`
* `write_block_doc`
* `setup_build`

Task descriptions are kept separate from the Rust implementation and are
converted into generated Rust metadata during the build.

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
  4. automatic summaries of Git history
* automatic tests, or at least initial test stubs

Independent of these goals, automatic code-fix runs are already supported,
but they are not the main objective.

## Dependencies

* [struct_extractors](https://github.com/witold-k/struct_extractors)
* [fsscanner](https://github.com/witold-k/fsscanner)

Future underlying component:

* [TokenDB](https://github.com/witold-k/token_db)

