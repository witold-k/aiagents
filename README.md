# AI Agents - An experimental agent runtime for AI-assisted software engineering

- **ATTENTION** this repository is under active development
- **ATTENTION** use at own risk, currently test coverage is low
- BUT aleady experinally usable, see section examples/howto

An experimental Rust-based agent runtime for integrating LLMs into
software engineering workflows.

The project explores how an LLM can act as an engineering agent that
can inspect source code, use controlled tools, modify a project and
iteratively react to build, lint and test results.

The central idea is a feedback loop:

    Task
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
      +-- success --> done
      |
      +-- failure --> feedback
                         |
                         v
                       LLM

The runtime is implemented in Rust and provides a controlled tool
interface between the LLM and the software project.

## Features

* LLM-driven software engineering workflows
* Iterative build / analyze / modify / test loop
* Source tree and file inspection
* AST-based source analysis
* Controlled file modification
* Build, lint and test feedback
* Configurable LLM providers
* OpenAI-compatible chat completion API
* Task-specific prompts and workflows
* Workspace support
* Path-based access restrictions
* Rust implementation
* GitHub Actions CI
* Test coverage generation

## Architecture

The core of the project is the `AIAgentLoop`.

It maintains the current task context and provides the LLM with:

* task description
* optional subtasks
* project structure
* relevant files
* build or test failures
* optional notes and focus information

The LLM response is interpreted as an action and dispatched to one of the
available agent tools.

The agent can then continue the interaction based on the tool result.

This creates an iterative engineering loop instead of a single
request/response interaction.

## Agent Tools

The agent currently provides explicit tools for operations such as:

* directory listing
* directory scanning
* file loading
* partial file loading
* file modification
* partial file modification
* AST inspection
* notes
* focus management
* workflow completion and failure handling

The tool layer is intentionally separated from the LLM interface.

This makes the boundary between model-generated decisions and actual
operations explicit.

## Security Model

The agent is designed around restricted capabilities.

The LLM does not directly receive unrestricted access to the host system.

In particular, the tool layer is designed to prevent the agent from:

* accessing files outside configured paths
* modifying files outside the project
* directly interacting with version control systems
* escalating privileges
* invoking arbitrary tools outside the defined tool interface

The intention is to keep the model inside a controlled software-engineering
sandbox.

## Workflows

Tasks are mapped to different development workflows.

Examples include:

* code analysis
* code review
* code fixing
* code generation
* test generation
* documentation
* build setup
* code transpilation

Build-oriented workflows can execute a sequence such as:

```text
Build
  |
  v
Lint
  |
  v
Test
```

When an operation fails, the resulting output can be passed back into the
agent loop so that the LLM can diagnose and attempt a correction.

## Task Descriptions

Task descriptions are stored separately from the Rust implementation.

During the build, the task descriptions are converted into generated Rust
code containing:

* task identifiers
* task names
* task prompts
* task iteration helpers

This keeps the task definitions separate from the agent implementation while
still providing compile-time generated task metadata.

## LLM Providers

The provider configuration is intentionally generic.

The agent communicates with an OpenAI-compatible chat completion endpoint
and can therefore be used with different local or remote model servers.

The default development configuration uses a local model server, but the
provider configuration can be changed to another compatible endpoint.

Example configuration concepts include:

```json
{
  "name": "default",
  "endpoint": "http://localhost:8080/v1",
  "model": "your-model",
  "api_key": ""
}
```

The project is particularly suited to experimenting with locally hosted
coding models.

## Build

The project requires a recent stable Rust toolchain.

Build with:

```bash
cargo build
```

Run the tests with:

```bash
cargo test
```

Run Clippy with:

```bash
cargo clippy -- -D warnings
```

The repository also contains a GitHub Actions workflow that performs build,
test, Clippy and coverage checks.

## Command Line

The main executable is `aifix`.

Typical options include:

```text
aifix [option]+

-l, --lang        select the programming language
-t, --task        select the task
-s, --select      select files or directories
-c, --config      configuration file
-f, --pathfilter  allowed path
-b, --builddir    build directory
-w, --workspace   workspace mode
-r, --run         start a configured LLM provider
-d, --debug       enable debug output
-h, --help        show help
```

For the complete list of supported tasks and languages, use:

```bash
aifix --help
```

## Project Status

This repository is an experimental and evolving project.

The goal is not to provide another general-purpose chatbot or agent
framework, but to investigate practical AI-assisted software engineering
from an engineer's perspective.

Current development focuses on:

* reliable agent/tool interaction
* controlled source-code modification
* build and test feedback
* workflow design
* context management
* local LLM integration
* improving the reliability of autonomous coding tasks

The architecture is expected to evolve as these experiments continue.

## Design Goals

The project explores several questions:

1. How much of a software engineering task can an LLM perform reliably?
2. How should an agent receive source-code context efficiently?
3. How can file access and modification be safely constrained?
4. How should build and test failures be fed back into the agent?
5. How can different engineering tasks share the same agent infrastructure?
6. How can local LLMs be integrated into practical development workflows?

## Examples / Howto

Take a look in the folder `runtests/**`, it contains code that need to be fixed.

- First build this project and put the artefact/binary `aifix` to your path.
`~/bin` should be sufficient.

- Then start the service via `aifix -r default` this will not work since settings
  are not correct, but a default settings file will created ` ~/.config/aifix/config.json`.
  Modify it and setting your settings, than start again `aifix -r default`

- finally run the code fixing as an rust example, change to dir `runtests/cargo/aitestloop_simple`
  and start the codefixing with `just fix`.

- you can do simular for c++ and java.

## License

Apache License 2.0

