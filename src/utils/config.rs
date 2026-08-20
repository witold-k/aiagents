// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde::{Serialize, Deserialize};
use std::{fs, env, path::Path, path::PathBuf};


#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AIProvider {
    pub name: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub llmbin: String,
    pub llmmodeldir: PathBuf,
    pub llmparam: Vec<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AITaskProvider {
    pub task: String,
    pub provider: String,
    pub temperature: f32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DockerSettings {
    pub image_name: String,
    pub arguments: Vec<String>,
}

/// which provider with with temperature to select for a given task
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub docker_settings: DockerSettings,
    pub provider: String,
    pub providerlist: Vec<AIProvider>,
    pub taskproviderlist: Vec<AITaskProvider>,
    pub queue_length_max: usize,
    pub queue_length_save: usize,
    pub scanfilter: Vec<String>,
    pub writefilter: Vec<String>,
    pub readfilter: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            docker_settings: DockerSettings {
                image_name: "".to_string(),
                arguments: Vec::<String>::new()
            },
            provider: "default".into(),
            providerlist: vec![
                AIProvider {
                    name: "default".into(),
                    endpoint: "http://localhost:8080/v1".into(),
                    model: "qwen2.5-coder-14b-instruct-q5_k_m.gguf".into(),
                    api_key: "".into(),
                    llmbin: "llama-server".into(),
                    llmmodeldir: PathBuf::from("/data/ai/llm/"),
                    llmparam: vec![
                        "--ctx-size".to_string(), "49152".to_string(),
                        "--n-gpu-layers".to_string(), "999".to_string(),
                        "--port".to_string(), "8080".to_string(),
                        "--host".to_string(), "127.0.0.1".to_string(),
                        "--timeout".to_string(), "600".to_string(),
                        "--flash-attn".to_string(), "on".to_string(),
                        "--batch-size".to_string(), "1024".to_string(),
                        "--ubatch-size".to_string(), "512".to_string(),
                        "--cache-type-k".to_string(), "q8_0".to_string(),
                        "--cache-type-v".to_string(), "q8_0".to_string(),
                        "--predict".to_string(), "8192".to_string(),
                        "--no-mmap".to_string(),
                    ]
                }
            ],
            taskproviderlist: Vec::<AITaskProvider>::new(),
            queue_length_max: 14,
            queue_length_save: 1,
            scanfilter: vec![
                "**/*.c".to_string(),
                "**/*.cc".to_string(),
                "**/*.cpp".to_string(),
                "**/*.cxx".to_string(),
                "**/*.h".to_string(),
                "**/*.hh".to_string(),
                "**/*.hpp".to_string(),
                "**/*.hxx".to_string(),
                "**/*.in".to_string(),
                "**/*.java".to_string(),
                "**/*.rs".to_string(),
                "**/*.sv".to_string(),
                "**/*.tcl".to_string(),
                "CMakeLists.txt".to_string(),
                "Cargo.toml".to_string(),
                "meson.build".to_string(),
                "pom.xml".to_string(),
            ],
            writefilter: vec![
                "{{projectdir}}".to_string(),
            ],
            readfilter: vec![
                "{{projectdir}}/..".to_string(),
                "/opt".to_string(),
            ],
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Config {
        let data = fs::read_to_string(path)
            .expect("Failed to read config.json");

        serde_json::from_str::<Config>(&data)
            .expect("Failed to parse config.json")
    }

     pub fn save(&self, path: &Path) {
        let json = serde_json::to_string_pretty(self)
            .expect("Failed to serialize config");

        // 1. Verzeichnis erstellen falls nötig
        if let Some(parent) = path.parent() &&
            let Err(e) = fs::create_dir_all(parent) {
                panic!(
                    "Failed to create directory {:?}. Code: {:?}. Reason: {}",
                    parent, e.raw_os_error(), e
                );
        }

        // 2. Datei schreiben
        if let Err(e) = fs::write(path, json) {
            panic!(
                "Failed to write {:?}. Code: {:?}. Reason: {}",
                path, e.raw_os_error(), e
            );
        }
    }

    /**
     * rturns true if created
     */
    pub fn ensure_exist(path: &Path) -> bool {
        if !path.exists() {
            Config::default().save(path);
            true
        }
        else {
            false
        }
    }

    pub fn load_or_create(path: Option<String>) -> Self {
        let path = match path {
            Some(path) => PathBuf::from(path),
            None       => Self::default_path()
        };
        if Self::ensure_exist(&path) {
            println!("Created default config at {:?}", path);
            Config::default()
        }
        else {
            Self::load(&path)
        }
    }

    pub fn default_path() -> PathBuf {
        if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg)
                .join("aifix")
                .join("config.json");
        }

        let home = env::var("HOME").expect("HOME not set");
        PathBuf::from(home)
            .join(".config")
            .join("aifix")
            .join("config.json")
    }

    pub fn get_provider(&self, name: &str) -> Option<AIProvider> {
        self.providerlist
            .iter()
            .find(|entry| entry.name == name)
            .cloned()
    }

    pub fn get_selected_provider(&self) -> Option<AIProvider> {
        self.get_provider(&self.provider)
    }
}
