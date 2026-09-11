# TODO

changed todos. repriorized. this is a hobby project - the way is the goal -
so removed boring tasks, kept fun or really necessary ones.
This todo list is ment top be a hint what to do and is addressed only to me.

## NEXT REDESIGN LLM calls/Buildsystem:

- workflow redesign

---

# Context and Source Analysis

## AST

For Rust source analysis, use a Rust `syn` (?) crate rather than an external AST
binary.

---

# Future Ideas

## LLM-Assisted Indexing

Explore LLM-based indexing of text blocks using keywords.

The longer-term goal could support both:

- human-oriented search
- LLM-oriented search

## Embeddings and SVD

Investigate semantic retrieval techniques such as embeddings and
singular-value decomposition (SVD).

Possible experiments include:

- normalized sliding windows over groups of words
- storing reduced representations
- retrieving similar problems and solutions
- decomposing a problem into several steps

## Web access

- use lightpanda for weblookup (maybe if ever ...)
[lightpanda](https://lightpanda.io/)

---

# Interactive Mode

An interactive mode is planned to allow users to work with the agent
conversationally during a development session.

The intended workflow is:

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

