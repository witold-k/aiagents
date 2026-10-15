// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use fsscanner::pathfilter::Pathfilter;
use symbol_to_source_resolver::Backend;
use crate::agenttools::aitooltype::{ResultToJson, ResultToString, Validatable};
use crate::agenttools::pathguard::can_read_resolved;
use crate::aimessageid::AIMessageId;

#[derive(Debug)]
pub struct LoadFileBySymbol<'a> {
    project_dir: PathBuf,
    filter: &'a Pathfilter,
    symbol: String,
}

#[derive(Debug, Clone)]
pub struct LoadFileBySymbolError(String);

#[derive(Debug, Clone)]
pub struct LoadFileBySymbolResult {
    pub data: String,
}

impl<'a> LoadFileBySymbol<'a> {
    pub fn from_json(
        projroot: &Path,
        filter: &'a Pathfilter,
        payload: &Value,
    ) -> Result<Self, LoadFileBySymbolError> {
        let symbol = payload.get("symbol").and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| LoadFileBySymbolError("missing non-empty 'symbol'".into()))?;

        Ok(Self {
            project_dir: projroot.to_path_buf(),
            filter,
            symbol: symbol.to_string(),
        })
    }

    pub fn execute(&self) -> Result<LoadFileBySymbolResult, LoadFileBySymbolError> {
        let backend = match (
            self.project_dir.join("Cargo.toml").exists(),
            self.project_dir.join("pom.xml").exists(),
            self.project_dir.join("compile_commands.json").exists(),
            self.project_dir.join("build/compile_commands.json").exists(),
        ) {
            (true, _, _, _) => Backend::CargoRust,
            (_, true, _, _) => Backend::MavenJava,
            (_, _, true, _) | (_, _, _, true) => Backend::CompileCommandsClang,
            _ => return Err(LoadFileBySymbolError(
                "no supported symbol resolver backend (Cargo.toml, pom.xml or compile_commands.json)".into()
            )),
        };

        // The resolver returns candidate file paths; it does not provide line ranges.
        // Access is checked before reading, including symlink resolution.
        let mut content = String::new();
        let candidates = match backend {
            Backend::CargoRust => {
                symbol_to_source_resolver::cargo_rust::CargoRustResolver::new(&self.project_dir)
                    .map_err(|err| LoadFileBySymbolError(err.to_string()))?
                    .resolve_files(&self.symbol)
                    .map_err(|err| LoadFileBySymbolError(err.to_string()))?
            }
            Backend::MavenJava => {
                symbol_to_source_resolver::maven_java::MavenJavaResolver::new(&self.project_dir)
                    .map_err(|err| LoadFileBySymbolError(err.to_string()))?
                    .resolve_files(&self.symbol)
                    .map_err(|err| LoadFileBySymbolError(err.to_string()))?
            }
            Backend::CompileCommandsClang => {
                symbol_to_source_resolver::compile_commands_clang::CompileCommandsClangResolver::new(if self.project_dir.join("compile_commands.json").exists() {
                    self.project_dir.join("compile_commands.json")
                } else {
                    self.project_dir.join("build/compile_commands.json")
                })
                    .map_err(|err| LoadFileBySymbolError(err.to_string()))?
                    .resolve_files(&self.symbol)
            }
        };
        for path in candidates {
            let path = path.canonicalize()
                .map_err(|err| LoadFileBySymbolError(format!("{}: {err}", path.display())))?;
            if !can_read_resolved(self.filter, &path) {
                continue;
            }
            let data = std::fs::read_to_string(&path)
                .map_err(|err| LoadFileBySymbolError(format!("{}: {err}", path.display())))?;
            content.push_str(&format!("=== {} ===\n{}\n", path.display(), data));
        }
        if content.is_empty() {
            content = format!("No readable source files found for symbol '{}'", self.symbol);
        }
        Ok(LoadFileBySymbolResult { data: content })
    }
}

impl fmt::Display for LoadFileBySymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[load_file_by_symbol] ERROR: {}", self.0)
    }
}
impl std::error::Error for LoadFileBySymbolError {}

impl ResultToJson for Result<LoadFileBySymbolResult, LoadFileBySymbolError> {
    fn to_json(&self, msg_id: AIMessageId) -> Value {
        let data = match self { Ok(result) => result.data.clone(), Err(err) => err.to_string() };
        json!({"role":"tool", "tool_call_id":msg_id, "content":data})
    }
}
impl ResultToString for Result<LoadFileBySymbolResult, LoadFileBySymbolError> {
    fn to_msg_string(&self, msg_id: AIMessageId) -> String {
        match self {
            Ok(result) => format!("{}: {}", msg_id, result.data),
            Err(err) => err.to_string(),
        }
    }
}
impl Validatable for Result<LoadFileBySymbolResult, LoadFileBySymbolError> {
    fn is_valid(&self) -> bool { self.is_ok() }
}
