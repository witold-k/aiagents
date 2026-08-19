# features

- in code tasks

- ast: use sym crate for rust

# optimazation

## refrag

- identify reusable components and make own project:
   - repostate
   - doprocess (necessary at all?)
   - stringutis
   - save, save_part => fsscanner (partly)

## supplychain security & code size

- remove crate phf, TOOLS soll in build.rs generiert werden
- remove crates: globset => use own fsscanner

# quality

- add tests
- add CI/CD for codeberg & github

# found issues

Priority	Change
P0	Add maximum agent iterations / time / LLM calls
P0	Add comprehensive filesystem/security tests
P0	Remove panic-prone unwrap() from LLM-controlled tool input
P1	Replace text-based "action" protocol with structured tool calls
P1	Preserve and propagate detailed errors
P1	Add no-progress / repeated-error detection
P1	Add unit + integration tests
P2	Improve context/retrieval architecture
P2	Decouple task definitions from generated code
P2	Add observability / execution traces
