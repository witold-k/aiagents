# QUICKSTART

You need:

- a running OpenAI API-compatible LLM service, e.g. via llama.cpp or Ollama
- a repository you want to work on
- and either:
  - a compiled version of `aifix` in your `PATH`
  - or a Rust installation and this repository to compile it yourself

## 1. Compile

From the repository root, install `aifix` with:

```bash
cargo install --path .
```

By default, Cargo installs binaries into `$CARGO_HOME/bin` (usually
`~/.cargo/bin`). Make sure that directory is in your `PATH`.

## 2. Set up configuration

Create a default configuration with:

```bash
aifix -r default
```

Then edit the generated configuration, for example:

```bash
nvim ~/.config/aifix/config.json
```

or:

```bash
nano ~/.config/aifix/config.json
```

Configure the endpoint and model for your OpenAI API-compatible LLM service.

## 3. Start the LLM service

Start your OpenAI API-compatible LLM service, for example llama.cpp or Ollama,
using the endpoint configured in `~/.config/aifix/config.json`.

## 4. Run aifix to fix code

Change to the repository you want to work on and run, for example:

```bash
aifix -l rust -t fix_code -f $(pwd) -f $(pwd)/..
```

Add extra instructions directly with `-m` (repeatable), without a separate
subtask file:

```bash
aifix -l rust -t fix_code -m "Keep the constructor private; fix callers to use the factory." -f "$(pwd)" -f "$(pwd)/.."
```

Alternatively, a second `-t path/to/instructions.md` loads additional task
instructions from a file. `-m` accepts literal text instead. Place these
options before `-a`, which consumes the remaining arguments.

For C++, configure the project first and provide its build directory:

```bash
aifix -l cpp -t fix_code -b <build-directory> -f $(pwd) -f $(pwd)/..
```

## 5. Examples

Small end-to-end examples can be found in `runtests`, for example:

```bash
cd runtests/cargo/aitestloop_simple
just fix
```

## LLM output token limit

Use `-k COUNT` or `--tokens COUNT` to set the maximum number of output tokens requested from the model. The default is `30000` and the value must be a positive integer.

```sh
aifix -t release_doc -k 8192
```

This option controls the LLM response token limit, **not** the input context window or the model server's context-size setting. Place global options before `-a/--args`, because `-a` passes all remaining arguments to the selected task.
