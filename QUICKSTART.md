# QUICKSTART

you need
- a running OpenAI API compatible LLM instance, e.g. via llama.cpp oder ollama.
- a repository you want to fix
- and either
    - a compiled version of `aifix` in your path, e.g. `~/bin`
    - or a rust installation and this repo to compile it yourself


## 1. compile

just type `cargo install`. `aifix` will be created and compiled to `~/bin`

## 2. setup configuration

just type

```bash
aifix -r default
```

to create a default configuration, you need to edit it, eg:
```bash
nvim ~/.config/aifix/config.json
```
or
```bash
nano ~/.config/aifix/config.json
```

## 3. start llm service

if you host your own llm you can start it after edit
the confuration file directly via:
```bash
aifix -r default
```

## 4. run aifix to fix code

change dir to the current code and execute, e.g.
```bash
aifix -l rust -t fix_code -f $(pwd) -f $(pwd)/..
```

if you have c++ you need to configure your project first, then:
```bash
aifix -l cpp -t fix_code -b <build directory> -f $(pwd) -f $(pwd)/..
```

## 5. examples

examples can be found in `runtests`, e.g.
```bash
cd runtests/cargo/aitestloop_simple
just fix
```

