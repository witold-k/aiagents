# features

- ast: use sym crate for rust

# optimazation

## refrag

- identify reusable components and make own project:
   - repostate
   - doprocess (necessary at all?)
   - stringutis
   - save, save_part => fsscanner (partly)
- in general review dependency/supply chain

## supplychain security & code size

- remove crate phf, TOOLS should be generated in build.rs
- remove crates: globset => use own fsscanner
- supply chain security:
    - may be do not use cargo to import crates (at least not all), instead just copy to subdir

# quality

- add unit tests
    - first at least all in utils
    - than next all in agenttools
    - later on repostate? => in the moment very simple

# security

- docker integration: provide simple docker based start defined in config file
    aifix (used as docker/podman starter) => starts docker with aifix again that runs in docker/podman

# next todos:

# AI Agents — Priority List

## P0 — Make the agent impossible to hang/crash

1. Remove every LLM-input `unwrap()` / `expect()`.
4. Add a wall-clock timeout.
5. Define explicit terminal states.

## P1 — Security

6. Add comprehensive path-traversal tests.
7. Add symlink tests.
8. Add absolute-path tests.
9. Add read/write capability tests.
10. Centralize path validation.

## P1 — Agent Protocol

12. Give every tool call an ID.
13. Separate `ToolResult` from wire-format messages.
14. Make malformed tool requests ordinary errors instead of panics.

## P1 — Evaluation

15. Build a small deterministic benchmark suite.

Example:

* 10 Rust bugs
* 10 C++ bugs
* 10 Java bugs

Measure:

* Task success
* Number of iterations
* Number of LLM calls
* Token usage
* Execution time
* Compiler failures
* Incorrect edits

## P2 — Context

16. Add symbol-level indexing.
17. Add AST-aware retrieval.
18. Add relevance scoring.
19. Only then experiment with embeddings.

## Major Milestone

Run **20 representative coding tasks unattended** and record exactly why each task succeeded or failed.

# future plans / ideas

## some topics

- llm based indexing in textblock => keywords
    - will be extended to human + llm search
- may be search via embeddings => SVD singular value decomposition
    - different SVD: normalized sliding window over word group (), ...
- may be store also SVD
    - problem (e.g. error message) => solution
    - may be problem can be decomposized in several steps?
- ast: use rust crate for this, do not use external binary

## workflows

- need more idea for workflows
current only very simple workflow type supported: linear execution with error or done as result.
need later on complex workflow with multi tool call, depend on current running internal task.
=> most probably more or several statemachine(s) needed here

## Interactive Mode

An interactive mode is planned to allow users to work with the agent conversationally during a development session.

The goal is to support an iterative workflow where the user can:

- give the agent a task
- inspect what the agent is doing
- provide additional instructions
- review proposed changes
- allow or reject actions
- continue the task interactively
- inspect build and test feedback

Implementation will be done via markdown documents - so no addition tooling is needed.
Just an editor that is may be aware of fileupdates. And on save actions should be detected
by other tool and forward text to agent, if user request is complete (may be keyword at and of markdown)
