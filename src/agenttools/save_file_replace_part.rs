// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use std::io::Write;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use struct_extractors::extract_accessors;
use fsscanner::{
    pathfilter::Pathfilter,
    pathutils::{normalize_path, resolve_relaxed_path},
};
use crate::agenttools::aitooltype::{ ResultToString, ResultToJson, Validatable };

#[extract_accessors]
#[derive(Debug)]
pub struct SaveFilePart<'a> {
    filter: &'a Pathfilter,
    path: PathBuf,
    occurrence_index: usize,
    #[access(get_ref)]
    content: String,
    #[access(get_ref)]
    original: String,
    #[access(get_ref)]
    note: String
}

#[derive(Copy, Clone, Debug)]
pub enum SaveFilePartErrorType {
    DecodeError,
    Forbidden,
    NotFound,
    ReadFailed,
    WriteFailed,
    OriginalMismatch,
}

#[derive(Clone, Debug)]
pub struct SaveFilePartError {
    err_type: SaveFilePartErrorType,
    err_info: String,
}

#[extract_accessors]
#[derive(Debug, Clone)]
pub struct SaveFilePartResult {
    pub occurrence_index: usize,
    #[access(get_ref)]
    content: String,
    #[access(get_ref)]
    original: String,
    #[access(get_ref)]
    note: String
}

// ---------------------------------------------------------------------------
// SaveFilePart implementation
// ---------------------------------------------------------------------------

impl<'a> SaveFilePart<'a> {
    pub fn from_json(
        projroot: &'a Path,
        filter: &'a Pathfilter,
        payload: &'a Value,
    ) -> Result<SaveFilePart<'a>, SaveFilePartError> {
        let file = match payload.get("file").and_then(|v| v.as_str()).map(PathBuf::from) {
            Some(file) => file,
            None => {
                return Err(SaveFilePartError {
                    err_type: SaveFilePartErrorType::DecodeError,
                    err_info: format!(
                        "invalid file: {}",
                        payload.get("file").unwrap_or(&Value::Null),
                    ),
                });
            }
        };

        let rpath = match resolve_relaxed_path(projroot, &file) {
            Some(rpath) => rpath,
            None => {
                return Err(SaveFilePartError {
                    err_type: SaveFilePartErrorType::DecodeError,
                    err_info: format!("invalid file path: {}", file.display()),
                });
            }
        };

        let occurrence_index = match payload
            .get("index")
            .or_else(|| payload.get("occurrence"))
            .and_then(|v| v.as_u64())
        {
            Some(index) => index as usize,
            None => {
                return Err(SaveFilePartError {
                    err_type: SaveFilePartErrorType::DecodeError,
                    err_info: format!(
                        "invalid occurrence index: {}",
                        payload
                            .get("index")
                            .or_else(|| payload.get("occurrence"))
                            .unwrap_or(&Value::Null),
                    ),
                });
            }
        };

        let original = match payload.get("original").and_then(|v| v.as_str()) {
            Some(original) => original,
            None => {
                return Err(SaveFilePartError {
                    err_type: SaveFilePartErrorType::DecodeError,
                    err_info: format!(
                        "invalid original: {}",
                        payload.get("original").unwrap_or(&Value::Null),
                    ),
                });
            }
        };

        let content = match payload.get("content").and_then(|v| v.as_str()) {
            Some(content) => content,
            None => {
                return Err(SaveFilePartError {
                    err_type: SaveFilePartErrorType::DecodeError,
                    err_info: format!(
                        "invalid content: {}",
                        payload.get("content").unwrap_or(&Value::Null),
                    ),
                });
            }
        };

        let note = match payload.get("note").and_then(|v| v.as_str()) {
            Some(note) => note,
            None => {
                return Err(SaveFilePartError {
                    err_type: SaveFilePartErrorType::DecodeError,
                    err_info: format!(
                        "invalid note: {}",
                        payload.get("note").unwrap_or(&Value::Null),
                    ),
                });
            }
        };

        Ok(SaveFilePart {
            filter,
            path: normalize_path(&rpath),
            occurrence_index,
            original: original.to_string(),
            content: content.to_string(),
            note: note.to_string(),
        })
    }

    pub fn execute(self) -> Result<SaveFilePartResult, SaveFilePartError> {
        let path = normalize_path(&self.path);

        if !self.filter.contains(&path) {
            return Err(SaveFilePartError::new(SaveFilePartErrorType::Forbidden, &path, ""));
        }

        if !path.exists() {
            return Err(SaveFilePartError::new(SaveFilePartErrorType::NotFound, &path, ""));
        }

        // Read full file content
        let file_content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => return Err(SaveFilePartError::new(SaveFilePartErrorType::ReadFailed, &path, "")),
        };

        // Normalisiere Zeilenumbrüche für einen robusten String-Match
        let clean_file = file_content.replace("\r\n", "\n");
        let clean_original = self.original.replace("\r\n", "\n");
        let clean_content = self.content.replace("\r\n", "\n");

        // Finde alle Startpositionen des "original"-Blocks im File
        let mut matches = Vec::new();
        let mut start_pos = 0;
        while let Some(pos) = clean_file[start_pos..].find(&clean_original) {
            let absolute_pos = start_pos + pos;
            matches.push(absolute_pos);
            start_pos = absolute_pos + clean_original.len();
        }

        // Check 1: Der Block existiert überhaupt nicht im File
        if matches.is_empty() {
            let err = format!(
                "Original mismatch: The provided 'original' block was not found anywhere in the file.\n\
                - Your requested block:\n\"\"\"\n{}\n\"\"\"",
                clean_original
            );
            return Err(SaveFilePartError::new(SaveFilePartErrorType::OriginalMismatch, &path, &err));
        }

        // Check 2: Der gewünschte Index existiert nicht (z.B. Index 2 gefordert, aber nur 2 Matches [0, 1] vorhanden)
        if self.occurrence_index >= matches.len() {
            let err = format!(
                "Original mismatch: Index out of bounds.\n\
                - You requested index: {} (0-based)\n\
                - Total identical matches found in file: {}\n\
                - Reduce the index or make your 'original' block larger/more unique.",
                self.occurrence_index, matches.len()
            );
            return Err(SaveFilePartError::new(SaveFilePartErrorType::OriginalMismatch, &path, &err));
        }

        // Identifiziere die exakte Byte-Position für das gewählte Vorkommen
        let target_start_byte = matches[self.occurrence_index];
        let target_end_byte = target_start_byte + clean_original.len();

        // Rekonstruiere das File mit dem Ersetzten Block
        let mut updated_content = String::with_capacity(clean_file.len() + clean_content.len());
        updated_content.push_str(&clean_file[..target_start_byte]);
        updated_content.push_str(&clean_content);
        updated_content.push_str(&clean_file[target_end_byte..]);
        // -----------------------------------------------------

        // Write back to file
        match OpenOptions::new().write(true).truncate(true).open(&path) {
            Ok(mut file) => {
                if file.write_all(updated_content.as_bytes()).is_err() {
                    return Err(SaveFilePartError::new(SaveFilePartErrorType::WriteFailed, &path, ""));
                }
                Ok(SaveFilePartResult {
                    occurrence_index: self.occurrence_index, // In diesem Modus optional/nicht zeilenbasiert
                    content: updated_content,
                    original: self.original,
                    note: self.note
                })
            },
            Err(_) => Err(SaveFilePartError::new(SaveFilePartErrorType::WriteFailed, &path, "")),
        }
    }
}

// ---------------------------------------------------------------------------
// SaveFilePartResult JSON
// ---------------------------------------------------------------------------

impl SaveFilePartResult {
    pub fn to_json(&self, message_id: &str) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id,
            "content": self.content,
        })
    }

    pub fn to_string(&self, message_id: &str) -> String {
        format!("{}: {}", message_id, self.content)
    }
}

// ---------------------------------------------------------------------------
// SaveFilePartError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for SaveFilePartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.err_type, self.err_info)
    }
}

impl std::error::Error for SaveFilePartError {}

impl SaveFilePartError {
    pub fn new(err_type: SaveFilePartErrorType, path: &Path, info: &str) -> Self {
        let err_info = format!("{}: {}", path.to_string_lossy(), info);
        Self { err_type, err_info }
    }

    pub fn to_json(&self, message_id: &str) -> Value {
        let msg = match self.err_type {
            SaveFilePartErrorType::DecodeError =>
                format!("[save_file] ERROR: decode error: {}", self.err_info),

             SaveFilePartErrorType::Forbidden =>
                format!("[save_file] ERROR: not allowed to read: {}", self.err_info),

            SaveFilePartErrorType::NotFound =>
                format!("[save_file] ERROR: file not found: {}", self.err_info),

            SaveFilePartErrorType::ReadFailed =>
                format!("[save_file] ERROR: read (before write) failed: {}", self.err_info),

            SaveFilePartErrorType::WriteFailed =>
                format!("[save_file] ERROR: write failed: {}", self.err_info),

            SaveFilePartErrorType::OriginalMismatch =>
                format!("[save_file] ERROR: original mismatch: {}", self.err_info),
        };

        json!({
            "role": "tool",
            "tool_call_id": message_id,
            "content": msg,
        })
    }
}

// ---------------------------------------------------------------------------
// ResultToJson implementation
// ---------------------------------------------------------------------------

impl ResultToJson for Result<SaveFilePartResult, SaveFilePartError> {
    fn to_json(&self, msg_id: &str) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<SaveFilePartResult, SaveFilePartError> {
    fn to_string(&self, msg_id: &str) -> String {
        match self {
            Ok(ok) => ok.to_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<SaveFilePartResult, SaveFilePartError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.is_ok()
    }
}

