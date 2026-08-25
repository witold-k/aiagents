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
    - may be du not use cargo to import crates (at least not all), instead just copy to subdir

# quality

- add unit tests
    - first at least all in utils
    - than next all in agenttools
    - later on repostate? => in the moment very simple

# security

- docker integration: provide simple docker based start defined in config file
    aifix (used as docker/podman starter) => starts docker with aifix again that runs in docker/podman

# found issues

Priority	Change
P0	Add comprehensive filesystem/security tests
P0	Remove panic-prone unwrap() from LLM-controlled tool input
P1	Preserve and propagate detailed errors
    - provide a simple logging abstraction, forward trace direct do stdout as first attempt
P2	Improve context/retrieval architecture
P2	Add observability / execution traces

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

