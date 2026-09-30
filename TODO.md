# TODO

This is a hobby project — the journey is the goal. This file is not intended as
a strict roadmap. It is a reminder of interesting or necessary places to resume
work after a pause.

## When returning to this project

1. Run and evaluate the current BLT repair workflow on real failures.
2. Improve workflow failure/result reporting where it is actually useful.
3. Revisit release-documentation generation and the quality of its Git context.
4. Pick one interesting experiment rather than following a fixed roadmap:
   indexing/search, TokenDB, SVD/embeddings, or interactive mode.

---

# Context and Source Analysis

## AST

For Rust source analysis, use the Rust `syn` crate rather than an external AST
binary.

---

# Future Ideas

## Local indexing and search

Experiment with indexing source code and documents so useful context can be
found without simply loading more files into the LLM context.

Possible directions include:

- keyword or LLM-assisted indexing of text blocks
- human-oriented search
- LLM-oriented context retrieval
- persistent local indexes
- combining exact/full-text retrieval with semantic methods

## TokenDB

Explore whether `token_db` is useful as a persistent vocabulary/corpus layer
for indexing and retrieval experiments.

## Embeddings and SVD

Investigate semantic retrieval techniques such as embeddings and
singular-value decomposition (SVD).

Possible experiments include:

- normalized sliding windows over groups of words
- storing reduced representations
- retrieving similar problems and solutions
- decomposing a problem into several steps
- comparing simple lexical retrieval, SVD/LSA, and embeddings on real data

## Release documentation

The basic Git-history-to-`RELEASE_NOTES.md` workflow exists. Interesting future
work is therefore about quality rather than merely implementing it:

- decide which Git context is actually useful to the model
- evaluate summaries on real release ranges
- keep generated release notes concise and technically meaningful
- consider whether diffs, complete files, or a mixture provide better context

## Web access

- use Lightpanda for web lookup (maybe, if ever ...)
[Lightpanda](https://lightpanda.io/)

---

# Interactive Mode

An interactive mode remains an idea for allowing users to work with the agent
conversationally during a development session. It is not necessarily the next
thing to implement.

The intended workflow could be:

- give the agent a task
- inspect what the agent is doing
- provide additional instructions
- review proposed changes
- allow or reject actions
- continue the task interactively
- inspect build and test feedback

The initial implementation should use Markdown documents rather than
introducing another dedicated UI or protocol.

A Markdown document can act as the interaction surface:

    User/editor
        |
        v
    Markdown document
        |
        v
    file change detected
        |
        v
    agent receives new text
        |
        v
    agent continues

The editor only needs to support normal file editing and saving.

An external watcher/tool can detect changes and forward the updated text
to the agent.

A simple convention, such as a keyword at the end of the Markdown document,
can indicate that the user's request is complete and should be processed.

This keeps interactive mode consistent with the project's goal of using
simple existing mechanisms rather than adding another tooling layer.
