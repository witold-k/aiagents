# AI Agents - An experimental agent runtime for AI-assisted software engineering

- **ATTENTION:** this repository is under active development.
- **ATTENTION:** use at your own risk; test coverage is currently low.
- **BUT:** it is already experimentally usable; see [Examples / Howto](#examples--howto).

An experimental Rust-based agent runtime for integrating LLMs into software
engineering workflows.

The project explores how LLM-based agents can inspect and modify software
projects using controlled tools, verification steps, and iterative feedback.

The basic idea is:

```
    Workflow
      |
      v
    LLM Agent
      |
      +-- inspect
      +-- analyze
      +-- modify
      |
      v
    Build / Lint / Test
      |
      +-- success --> next workflow step
      |
      +-- failure --> agent feedback
```

The runtime is implemented in Rust and provides a controlled interface
between LLMs and software projects.

## Features

* LLM-driven software engineering workflows
* Multiple-agent workflow support
* Iterative build / analyze / modify / test loops
* Source tree, file and AST inspection
* Controlled file modification
* Build, lint and test feedback
* Repository state and recovery
* Configurable LLM providers
* Workspace and path-based access restrictions
* Rust implementation

## Architecture

The project separates **workflow orchestration** from the execution of an
individual LLM agent.

A workflow defines the larger task and decides what happens next. It may
coordinate several agents, verification steps, or repository recovery.

For example:

```
    Codify
      |
      v
    Build / Lint / Test
      |
      v
    Review
      |
      v
    Change
      |
      v
    Build / Lint / Test
      |
      v
    Documentation
      |
      v
    Done
```

`AIAgentLoop` provides the runtime for an individual LLM-driven agent. It
handles the interaction between the model, its context, and the available
agent tools.

The workflow is more abstract than an individual agent loop. It defines
how agents and other operations are combined to accomplish a larger task
and decides what should happen next.

The architecture is intentionally kept simple and is expected to evolve as
different workflow patterns are explored.

## Agent Tools

Agents interact with the project through explicit tools for operations such
as:

* directory and file inspection
* partial file loading
* file modification
* AST inspection
* notes and focus management
* workflow completion and failure handling

The tool layer separates model-generated decisions from actual operations.

## Repository State

`RepoState` is responsible for repository recovery.

It is an independent domain concept and has no dependency on the LLM or
source-code inspection.

Its purpose is to maintain and restore repository state when a coding task
needs to be recovered from a broken or unwanted change.

The current implementation provides a simple recovery mechanism. The
intention is to support stateful recovery as workflows become more capable.

## Workflows

Workflows provide the higher-level orchestration for software-engineering
tasks.

A workflow can coordinate multiple agents, verification steps, repository
recovery, and other operations. It can use different agents or LLM
configurations for different steps and can choose the next state based on
the outcome of previous steps.

For example, a workflow might perform:

```
    Codify
      |
      v
    Build / Lint / Test
      |
      +-- failure --> Fix
      |
      +-- success
            |
            v
          Review
            |
            v
        Documentation
            |
            v
           Done
```

Build/lint/test is deliberately kept simple. It verifies the current
project state and reports the result to the workflow.

New workflow abstractions should only be introduced when required by actual
use cases.

## Task Descriptions

Task descriptions are stored separately from the Rust implementation.

During the build, they are converted into generated Rust code containing:

* task identifiers
* task names
* task prompts
* task iteration helpers

This keeps task definitions separate from the agent implementation while
providing compile-time task metadata.

## LLM Providers

The provider configuration is generic and uses an OpenAI-compatible chat
completion endpoint, allowing local or remote model servers to be used.

Example:

    {
      "name": "default",
      "endpoint": "http://localhost:8080/v1",
      "model": "your-model",
      "api_key": ""
    }

The project is particularly suited to experimenting with locally hosted
coding models.

## Security Model

The agent operates through restricted capabilities rather than unrestricted
host access.

The tool layer is designed to prevent the agent from:

* accessing files outside configured paths
* modifying files outside the project
* directly interacting with version control systems
* escalating privileges
* invoking arbitrary tools outside the defined interface

## Build

The project requires a recent stable Rust toolchain.

```bash
    cargo build
    cargo test
    cargo clippy -- -D warnings
```

GitHub Actions also performs build, test, Clippy and coverage checks.

## Command Line

The main executable is `aifix`.

```bash
    aifix [option]+
    -l --lang [required, invalid with -w switch]: select task: one of: generic, verilog, java, cpp, rust
            or provide path to taskdesciption
    -t --task [required, multiple possible invalid with -w switch]:
        - first -t selects task: one of: write_item_doc, write_module_doc, review_code, analyze, setup_build, build, fix_code, review_doc, write_block_doc, gen_code, transpile_code, write_test_code
        - following are paths to subtasks that enhance the task description
    -s --select [optional, multiple possible]: one or more files or dirs,
        - if task needs a file to operate and none is given a random file will be choosen
    -c --config [optional]: load config from path
        - default config will be generated in path if no config availabe
        - default config path is ~/.config/aifix/config.json
    -f --pathfilter [multiple, required at least once]: directory list
    -b --builddir [default = target(rust) or build(other)]: set builddir
    -w --workspace [optional]: running in workspace (mode): path to workspace
        - llm does not load files, all files are loaded at once from current workspace,
        - also task descripton is here and may be named like e.g.: `task.md`
    -d --debug [default = false]: dump debug
    -r --run [optional providername in config] run server; if set other settings will be ignored
    -h --help [default = false]: dump help
```

For the complete list:

```bash
    aifix --help
```

## Project Status

This is an experimental and evolving project. The goal is not to provide
another general-purpose chatbot or agent framework, but to investigate
practical AI-assisted software engineering.

Current development focuses on:

* reliable agent/tool interaction
* workflow design and orchestration
* controlled source-code modification
* build and test feedback
* repository recovery
* local LLM integration
* keeping the runtime and its dependencies simple

The architecture is expected to evolve as these experiments continue.

## Design Goals

The project explores:

1. How reliably can LLM-based agents perform software engineering tasks?
2. How should source-code context be provided efficiently?
3. How can file access and modification be safely constrained?
4. How should verification failures be fed back into an agent?
5. How can different agents and LLM configurations participate in one workflow?
6. How can the runtime remain small and understandable?

## Examples / Howto

The `runtests/**` directory contains code that needs to be fixed.

First build the project and put the `aifix` binary somewhere in your path.
`~/bin` should be sufficient.

Start the service:

```bash
    aifix -r default
```

If the configuration does not exist, a default file will be created at:

```bash
    ~/.config/aifix/config.json
```

Modify it with your settings and start the service again:

```bash
    aifix -r default
```

Then change to:

```bash
    runtests/cargo/aitestloop_simple
```

and run:

```bash
    just fix
```

Similar examples are available for C++ and Java.

## License

Apache License 2.0

