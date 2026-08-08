# features

- in code tasks

- ast: use sym crate for rust

# optimazation

## refrag

- identify reusable components and make own project:
   - repostate
   - doprocess (necessary at all?)
   - stringutis
   - pathutils => fsscanner
   - pathfiler => fsscanner
   - save, save_part => fsscanner (partly)
   - fileentry => fsscanner ?

## supplychain security & code size

- remove crate phf, TOOLS soll in build.rs generiert werden
- remove crates: globset, walkdir => use own fsscanner

# quality

- add tests
- add CI/CD for codeberg & github
