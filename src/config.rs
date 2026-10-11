// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde::{Deserialize, Serialize};
use crate::pathfragments::expand_current_path_fragments;
use std::{
    env,
    fs,
    io,
    path::{Path, PathBuf},
};

/// Model API provider configuration, including optional local-server startup settings.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AIProvider {
    /// Unique name used to identify this provider.
    pub name: String,
    /// Whether this provider is enabled for selection.
    pub active: bool,
    /// Human-readable notes about the provider; informational only.
    pub comment: String,
    /// Source URL for obtaining or identifying the model; informational metadata.
    pub source: String,
    /// URL of the model inference API, typically an OpenAI-compatible chat endpoint.
    pub endpoint: String,
    /// Model identifier used by the provider, often the local model filename.
    pub model: String,
    /// Authentication key for API requests; may be empty for local servers.
    pub api_key: String,
    /// Allow insecure TLS verification for HTTPS connections to this provider.
    pub insecure: bool,
    /// Executable used to start a local model server, e.g. `llama-server`.
    pub llmbin: String,
    /// Directory containing the model file used by the local server.
    pub llmmodeldir: PathBuf,
    /// Additional command-line arguments passed to the local server executable.
    pub llmparam: Vec<String>,
}

/// Per-task override for provider selection and model sampling temperature.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AITaskProvider {
    /// Identifier of the task to which this override applies.
    pub task: String,
    /// Name of the provider assigned to this task.
    pub provider: String,
    /// Sampling temperature used when running the task.
    pub temperature: f32,
}

/// Failure limits applied while running an agent workflow.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Boundaries {
    /// Maximum allowed workflow-level failures.
    pub max_workflow_fail: usize,
    /// Maximum allowed tool-call failures.
    pub max_tool_call_fail: usize,
}

/// Main configuration for model providers, execution limits, scanning, and file access.
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    /// Name of the default provider.
    pub provider: String,
    /// Configured model providers available to the application.
    pub providerlist: Vec<AIProvider>,
    /// Optional task-specific provider and temperature settings.
    pub taskproviderlist: Vec<AITaskProvider>,
    /// Failure limits for workflows and tool calls.
    pub max_try_count: Boundaries,
    /// Maximum number of entries retained in the message/history queue.
    pub queue_length_max: usize,
    /// Number of recent queue entries preserved when the queue is trimmed.
    pub queue_length_save: usize,
    /// Filename suffixes accepted by source scanning (e.g. `.rs`, `.cpp`).
    /// Matches file endings, unlike `scanfullfilter`, which lists exact filenames.
    /// This controls scan selection, not read/write permissions.
    pub scanendfilter: Vec<String>,
    /// Complete filenames accepted by source scanning (e.g. `Cargo.toml`).
    /// Useful for build/configuration files without a common source-code suffix.
    /// This controls scan selection, not read/write permissions.
    pub scanfullfilter: Vec<String>,
    /// Filesystem roots within which agent write operations are permitted.
    /// Path fragments such as `{{projectdir}}` are expanded when loading.
    pub writefilter: Vec<String>,
    /// Filesystem roots within which agent read operations are permitted.
    /// Path fragments such as `{{projectdir}}` are expanded when loading.
    pub readfilter: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: "devs".into(),

            providerlist: vec![
                AIProvider {
                    name: "qwen".into(),
                    active: true,
                    comment: "works, tested on runtests".into(),
                    source: "https://huggingface.co/apto-as/Qwen2.5-Coder-14B-Instruct-Q5_K_M-GGUF/resolve/main/qwen2.5-coder-14b-instruct-q5_k_m.gguf".into(),
                    endpoint: "http://localhost:8080/v1/chat/completions".into(),
                    model: "qwen2.5-coder-14b-instruct-q5_k_m.gguf".into(),
                    api_key: String::new(),
                    insecure: false,
                    llmbin: "llama-server".into(),
                    llmmodeldir: PathBuf::from("/data/ai/llm/"),
                    llmparam: vec![
                        "--ctx-size".into(),
                        "49152".into(),
                        "--n-gpu-layers".into(),
                        "999".into(),
                        "--parallel".into(),
                        "1".into(),
                        "--port".into(),
                        "8080".into(),
                        "--host".into(),
                        "127.0.0.1".into(),
                        "--timeout".into(),
                        "600".into(),
                        "--flash-attn".into(),
                        "on".into(),
                        "--batch-size".into(),
                        "2048".into(),
                        "--ubatch-size".into(),
                        "1024".into(),
                        "--cache-type-k".into(),
                        "q8_0".into(),
                        "--cache-type-v".into(),
                        "q8_0".into(),
                        "--predict".into(),
                        "8192".into(),
                        "--load-mode".into(),
                        "mmap".into(),
                    ],
                },
                AIProvider {
                    name: "qwen38q3".into(),
                    active: false,
                    comment: "model can not fix code with current aifix".into(),
                    source: "https://huggingface.co/unsloth/Qwen3.8-27B-GGUF/blob/main/Qwen3.8-27B-UD-Q3_K_XL.gguf".into(),
                    endpoint: "http://localhost:8080/v1/chat/completions".into(),
                    model: "Qwen3.8-27B-UD-Q3_K_XL.gguf".into(),
                    api_key: String::new(),
                    insecure: false,
                    llmbin: "llama-server".into(),
                    llmmodeldir: PathBuf::from("/data/ai/llm/"),
                    llmparam: vec![
                        "--ctx-size".into(),
                        "80000".into(),
                        "--n-gpu-layers".into(),
                        "999".into(),
                        "--parallel".into(),
                        "1".into(),
                        "--port".into(),
                        "8080".into(),
                        "--host".into(),
                        "127.0.0.1".into(),
                        "--timeout".into(),
                        "600".into(),
                        "--flash-attn".into(),
                        "on".into(),
                        "--batch-size".into(),
                        "2048".into(),
                        "--ubatch-size".into(),
                        "1024".into(),
                        "--cache-type-k".into(),
                        "q8_0".into(),
                        "--cache-type-v".into(),
                        "q8_0".into(),
                        "--predict".into(),
                        "8192".into(),
                        "--reasoning-preserve".into(),
                        "--load-mode".into(),
                        "mmap".into(),
                    ],
                },
                AIProvider {
                    name: "qwen38q4, tested on runtests".into(),
                    active: true,
                    comment: "works, not tested with bigger problems".into(),
                    source: "https://huggingface.co/unsloth/Qwen3.8-27B-GGUF/resolve/main/Qwen3.8-27B-UD-IQ4_XS.gguf".into(),
                    endpoint: "http://localhost:8080/v1/chat/completions".into(),
                    model: "Qwen3.8-27B-UD-IQ4_XS.gguf".into(),
                    api_key: String::new(),
                    insecure: false,
                    llmbin: "llama-server".into(),
                    llmmodeldir: PathBuf::from("/data/ai/llm/"),
                    llmparam: vec![
                        "--ctx-size".into(),
                        "49152".into(),
                        "--n-gpu-layers".into(),
                        "999".into(),
                        "--parallel".into(),
                        "1".into(),
                        "--port".into(),
                        "8080".into(),
                        "--host".into(),
                        "127.0.0.1".into(),
                        "--timeout".into(),
                        "600".into(),
                        "--flash-attn".into(),
                        "on".into(),
                        "--batch-size".into(),
                        "2048".into(),
                        "--ubatch-size".into(),
                        "1024".into(),
                        "--cache-type-k".into(),
                        "q8_0".into(),
                        "--cache-type-v".into(),
                        "q8_0".into(),
                        "--predict".into(),
                        "8192".into(),
                        "--reasoning-preserve".into(),
                        "--load-mode".into(),
                        "mmap".into(),
                    ],
                },
                AIProvider { // just as an example - is very slow with 16GB VRAM
                             // => CPU offloading
                    name: "qwen38q5".into(),
                    active: false,
                    comment: "not useful, ca 2tok/s needs CPU offloading".into(),
                    source: "https://huggingface.co/unsloth/Qwen3.8-27B-GGUF/resolve/main/Qwen3.8-27B-UD-Q5_K_M.gguf".into(),
                    endpoint: "http://localhost:8080/v1/chat/completions".into(),
                    model: "Qwen3.8-27B-UD-Q5_K_M.gguf".into(),
                    api_key: String::new(),
                    insecure: false,
                    llmbin: "llama-server".into(),
                    llmmodeldir: PathBuf::from("/data/ai/llm/"),
                    llmparam: vec![
                        "--ctx-size".into(),
                        "100000".into(),
                        "--n-gpu-layers".into(),
                        "30".into(),
                        "--parallel".into(),
                        "1".into(),
                        "--port".into(),
                        "8080".into(),
                        "--host".into(),
                        "127.0.0.1".into(),
                        "--timeout".into(),
                        "600".into(),
                        "--flash-attn".into(),
                        "on".into(),
                        "--batch-size".into(),
                        "2048".into(),
                        "--ubatch-size".into(),
                        "1024".into(),
                        "--cache-type-k".into(),
                        "q8_0".into(),
                        "--cache-type-v".into(),
                        "q8_0".into(),
                        "--predict".into(),
                        "8192".into(),
                        "--spec-type".into(),
                        "draft-mtp".into(),
                        "--spec-draft-n-max".into(),
                        "2".into(),
                        // here CPU offloading is necessary
                    ],
                },

                AIProvider {
                    name: "devs".into(),
                    active: true,
                    comment: "works, tested on runtests".into(),
                    source: "https://huggingface.co/unsloth/Devstral-Small-2-24B-Instruct-2512-GGUF/resolve/main/Devstral-Small-2-24B-Instruct-2512-UD-Q4_K_XL.gguf".into(),
                    endpoint: "http://localhost:8080/v1/chat/completions".into(),
                    model: "Devstral-Small-2-24B-Instruct-2512-UD-Q4_K_XL.gguf".into(),
                    api_key: String::new(),
                    insecure: false,
                    llmbin: "llama-server".into(),
                    llmmodeldir: PathBuf::from("/data/ai/llm/"),
                    llmparam: vec![
                        "--ctx-size".into(),
                        "28000".into(),
                        "--n-gpu-layers".into(),
                        "999".into(),
                        "--parallel".into(),
                        "1".into(),
                        "--port".into(),
                        "8080".into(),
                        "--host".into(),
                        "127.0.0.1".into(),
                        "--timeout".into(),
                        "600".into(),
                        "--flash-attn".into(),
                        "on".into(),
                        "--batch-size".into(),
                        "2048".into(),
                        "--ubatch-size".into(),
                        "1024".into(),
                        "--cache-type-k".into(),
                        "q4_0".into(),
                        "--cache-type-v".into(),
                        "q4_0".into(),
                        "--predict".into(),
                        "8192".into(),
                        "--load-mode".into(),
                        "mmap".into(),
                    ],
                },
            ],

            taskproviderlist: Vec::new(),

            max_try_count: Boundaries {
                max_workflow_fail: 100,
                max_tool_call_fail: 100,
            },
            queue_length_max: 14,
            queue_length_save: 1,

            scanendfilter: vec![
                ".c".into(),
                ".cc".into(),
                ".cpp".into(),
                ".cxx".into(),
                ".h".into(),
                ".hh".into(),
                ".hpp".into(),
                ".hxx".into(),
                ".in".into(),
                ".java".into(),
                ".jl".into(), // julia
                ".rs".into(), // rust
                ".sv".into(), // system verilog
                ".tcl".into(),
            ],

            scanfullfilter: vec![
                "CMakeLists.txt".into(),
                "Cargo.toml".into(),
                "meson.build".into(),
                "pom.xml".into(),
            ],


            writefilter: vec![
                "{{projectdir}}".into(),
            ],

            readfilter: vec![
                "{{projectdir}}/..".into(),
                "/opt".into(),
            ],
        }
    }
}

impl Config {
    /// Expand configured filesystem roots for runtime use.
    /// The original configuration file is not modified.
    pub fn with_expanded_filters(mut self) -> Result<Self, String> {
        for filter in self.writefilter.iter_mut().chain(self.readfilter.iter_mut()) {
            *filter = expand_current_path_fragments(filter)?;
        }
        Ok(self)
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let data = fs::read_to_string(path)
            .map_err(|source| ConfigError::Read {
                path: path.to_path_buf(),
                source,
            })?;

        let config = serde_json::from_str::<Self>(&data)
            .map_err(|source| ConfigError::Parse {
                path: path.to_path_buf(),
                source,
            })?;

        config.validate()?;

        config.with_expanded_filters().map_err(ConfigError::Invalid)
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(ConfigError::Serialize)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|source| ConfigError::CreateDirectory {
                    path: parent.to_path_buf(),
                    source,
                })?;
        }

        fs::write(path, json)
            .map_err(|source| ConfigError::Write {
                path: path.to_path_buf(),
                source,
            })?;

        Ok(())
    }

    /// Returns `true` if the configuration file was created.
    pub fn ensure_exist(path: &Path) -> Result<bool, ConfigError> {
        if path.exists() {
            return Ok(false);
        }

        Self::default().save(path)?;
        Ok(true)
    }

    pub fn load_or_create(path: Option<String>) -> Result<Self, ConfigError> {
        let path = match path {
            Some(path) => PathBuf::from(path),
            None => Self::default_path(),
        };

        if Self::ensure_exist(&path)? {
            println!("Created default config at {:?}", path);
            Self::default().with_expanded_filters().map_err(ConfigError::Invalid)
        } else {
            Self::load(&path)
        }
    }

    pub fn default_path() -> PathBuf {
        if let Ok(xdg) = env::var("XDG_CONFIG_HOME")
            && !xdg.is_empty() {
                return PathBuf::from(xdg)
                    .join("aifix")
                    .join("config.json");
            }

        // we really need that => just abort here
        let home = env::var("HOME").expect("HOME not set");

        PathBuf::from(home)
            .join(".config")
            .join("aifix")
            .join("config.json")
    }

    pub fn get_provider(&self, name: &str) -> Option<&AIProvider> {
        if name == "default" {
            self.get_selected_provider()
        }
        else {
            self.providerlist
                .iter()
                .find(|entry| entry.name == name)
        }
    }

    pub fn get_selected_provider(&self) -> Option<&AIProvider> {
        self.get_provider(&self.provider)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        for (index, provider) in self.providerlist.iter().enumerate() {
            if self.providerlist[..index]
                .iter()
                .any(|other| other.name == provider.name)
            {
                return Err(ConfigError::Invalid(format!(
                    "duplicate provider name '{}'",
                    provider.name,
                )));
            }
        }

        if self.queue_length_save > self.queue_length_max {
            return Err(ConfigError::Invalid(
                "queue_length_save cannot be greater than queue_length_max"
                    .to_string(),
            ));
        }

        for task_provider in &self.taskproviderlist {
            if !self
                .providerlist
                .iter()
                .any(|provider| provider.name == task_provider.provider)
            {
                return Err(ConfigError::Invalid(format!(
                    "task '{}' references unknown provider '{}'",
                    task_provider.task,
                    task_provider.provider,
                )));
            }

            if !task_provider.temperature.is_finite() {
                return Err(ConfigError::Invalid(format!(
                    "task '{}' has a non-finite temperature",
                    task_provider.task,
                )));
            }
        }

        if !self.providerlist.iter().any(|p| p.name == self.provider) {
            return Err(ConfigError::Invalid(format!(
                "selected provider '{}' does not exist",
                self.provider,
            )));
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },

    Write {
        path: PathBuf,
        source: io::Error,
    },

    CreateDirectory {
        path: PathBuf,
        source: io::Error,
    },

    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },

    Serialize(serde_json::Error),

    Invalid(String),
}

