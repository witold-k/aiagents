# Ai readme

# **WARNING** README does not represent current state

aifix — Software Engineering Runtime

A small, auditable Rust runtime for AI-assisted software engineering.

# Quickstart

Please follow instructions in:
[Quickstart](QUICKSTART.md)

# What is it

«Generated code is untrusted code.»

"aifix" connects LLMs with software-engineering workflows such as code generation, repair, testing, documentation, review and transpilation.

The runtime is deliberately small. It does not give an LLM unrestricted access to the host system. Instead, it provides controlled tools and executes builds and tests in isolated, disposable environments.

«Status: experimental and under active development. Use at your own risk.»

The idea

"aifix" is not intended to be another interactive shell for an AI.

The basic workflow is:

```
             Workflow
                 |
                 v
             LLM Agent
                 |
        +--------+--------+
        |        |        |
      inspect  analyze   modify
                 |
                 v
          Build / Lint / Test
                 |
          +------+------+
          |             |
       success        failure
          |             |
          v             v
     next step       feedback
```

The important distinction is that the LLM decides what should be done, but the runtime decides what the agent is allowed to do and where generated code executes.

What it can do

- code generation and modification
- code repair
- build and test workflows
- test generation
- code review
- documentation generation
- source-code and AST inspection
- transpilation
- iterative build / analyze / modify / test loops
- multiple-agent workflows
- repository state and recovery
- configurable LLM providers
- local LLMs through OpenAI-compatible endpoints

The same runtime can be used for interactive development as well as automated software-maintenance workflows.

## Security model

"aifix" assumes that LLM-generated code may be actively malicious.

This matters because restricting the tools available to an agent is not enough.
An agent can generate a test containing arbitrary native code, and the test runner will execute it.

Therefore the security boundary is the execution environment, not the LLM prompt.

### Execution sandbox

Builds and tests are executed in a fresh, rootless Podman container.

```
aifix
  |
  v
LLM Agent
  |
  | generates / modifies code
  v
rootless Podman container
  |
  +-- workspace       RW
  +-- .git            inaccessible
  +-- network         disabled
  +-- host filesystem inaccessible
  +-- host credentials unavailable
  +-- no elevated capabilities
  |
  +-- build
  +-- test
  +-- generated code
  |
  v
result / diagnostics
  |
  v
container destroyed
```

A new execution environment is created for each build/test execution.
The environment is disposable: processes, temporary files and other state do not survive the execution.

The baseline execution model is intentionally simple:

- rootless Podman
- fresh container per execution
- no network by default
- no host credentials
- no privileged execution
- restricted mounts
- resource and execution limits
- container destroyed after execution

Stronger isolation, such as dedicated VMs or microVMs, can be provided by the deployment environment when required.

### Repository access

The normal workspace is mounted read/write because modifying the project is the purpose of the agent.

However, the Git repository metadata is not an agent capability.

If a repository is mounted as:

/repository

then ".git" is automatically hidden from the execution environment.

```
/repository          RW
/repository/src      RW
/repository/tests    RW
/repository/.git     inaccessible
```

Git operations are therefore exposed only through controlled runtime functionality rather than by giving generated code direct access to ".git".

This prevents generated code from directly modifying Git metadata, hooks, refs or objects.

Why the sandbox matters

Consider a generated test such as:

```
std::process::Command::new(...);
```

The relevant question is not whether the agent was given a "shell" tool.

The test itself is executable code.

Therefore "aifix" follows a simple rule:

«Do not trust the model. Do not trust generated code. Isolate execution instead.»

This keeps the security-critical part of the runtime small and auditable.

Architecture

The architecture separates workflow orchestration from an individual LLM agent.

```
Workflow
   |
   +-- Agent
   |     |
   |     +-- inspect
   |     +-- analyze
   |     +-- modify
   |
   +-- Build / Test
   |       |
   |       +-- isolated execution
   |
   +-- Review
   |
   +-- Recovery
   |
   v
 Done
```

"AIAgentLoop" provides the runtime for an individual LLM-driven agent.

A workflow defines the larger task and decides what happens next. It can coordinate multiple agents, verification steps and repository recovery.

The architecture is intentionally kept small. New abstractions should only be introduced when required by actual use cases.

Agent tools

Agents interact with projects through explicit operations such as:

- directory and file inspection
- partial file loading
- file modification
- AST inspection
- notes and focus management
- workflow completion and failure handling

The tool layer separates model-generated decisions from actual operations.

The agent does not receive an unrestricted host shell.

Repository state

"RepoState" is responsible for repository recovery.

It is independent of the LLM and source-code inspection.

Its purpose is to maintain and restore repository state when a coding task needs to be recovered from a broken or unwanted change.

LLM providers

The provider interface uses an OpenAI-compatible chat-completion endpoint.

This makes it possible to use either local or remote model servers.

Example:
```json
{
  "name": "default",
  "endpoint": "http://localhost:8080/v1",
  "model": "your-model",
  "api_key": ""
}
```

"aifix" is particularly suited to experimenting with locally hosted coding models.

No particular cloud provider is required by the runtime.

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

Build

Requires a recent stable Rust toolchain.

```
cargo build
cargo test
cargo clippy -- -D warnings
```

GitHub Actions also performs build, test, Clippy and coverage checks.

Command line

The main executable is:

`aifix`

The current CLI is task-oriented and experimental. The user-facing interface is expected to remain simple while the underlying workflow engine evolves.

Design principles

"aifix" is built around a few deliberately simple principles:

- 1. Generated code is untrusted.
- 2. The execution boundary enforces security.
- 3. Git metadata is not an agent capability.
- 4. Execution environments are disposable.
- 5. The runtime should remain small.
- 6. Local execution and local models are first-class use cases.
- 7. Complexity should only be introduced when a real workflow requires it.

The goal is not to build the largest agent framework.

The goal is to build a small, auditable runtime that can safely automate real software-engineering work.
