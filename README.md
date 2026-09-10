#

**WARNING** this project is under active development
**WARNING** code coverage / test count is low
**BUT** experital execution available, sede quickstart

# aiagents::aifix - experimental agtentic runtime

A small, auditable Rust runtime for AI-assisted software engineering ... or other tasks ...

## Quickstart

Please follow instructions in:
[Quickstart](QUICKSTART.md)

## What is it

`aifix` - the build binary is a small application that may be used for atomatic ai powered worklows. That may be:
- repair: build, test, fix_code
- create documentation
- review
- .. others, see [Tasks]
These worklows are already implemented. The same runtime can be used for interactive development
as well as automated software-maintenance workflows. The workflow concept is generic.

## Security model

### simplicity

well this is a secuirty for it own

### genral

Except file filters that disallow to write or read arbitary data. the are no other securies. This is by design.
The main design goal is "keep it simple and small". If you need additional security you may execute `aifix`
in docker, podman, vm or even a dedicated isolated pc. `aifix` ist just a binary without any depenencies.
It just need https access to a llm service. Thats all.

### Repository access

file access can be granted by `-f` command line switch. The first entry allows read/write access, all other read only.
If a repository is mounted as:

`/repository`

then ".git" is automatically hidden from the execution environment.

```
/repository          RW
/repository/src      RW
/repository/tests    RW
/repository/.git     inaccessible
```

Git operations are therefore exposed only through controlled runtime functionality rather than by giving generated code direct access to ".git".

This prevents generated code from directly modifying Git metadata, hooks, refs or objects.

## Tasks

The current task system includes operations such as:

- "analyze"
- "build"
- "fix_code"
- "gen_code"
- "write_test_code"
- "review_code"
- "review_doc"
- "transpile_code"
- "write_item_doc"
- "write_module_doc"
- "write_block_doc"
- "setup_build"

Task descriptions are kept separate from the Rust implementation and are converted into generated Rust metadata during the build.

