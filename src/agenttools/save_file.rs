// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde_json::{json, Value};
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use struct_extractors::extract_accessors;
use fsscanner::{
    pathfilter::Pathfilter,
    pathutils::{normalize_path, resolve_relaxed_path}
};
use crate::utils::jsonutils::get_json_field;
use crate::agenttools::aitooltype::{ AIToolType, ResultToString, ResultToJson, Validatable };

#[extract_accessors]
#[derive(Clone, Debug)]
pub struct SaveFile<'a> {
    filter: &'a Pathfilter,
    #[access(get_ref)]
    path: PathBuf,
    #[access(get_ref)]
    content: String,
    #[access(get)]
    note: &'a str
}

#[derive(Copy, Clone, Debug)]
pub enum SaveFileErrorType {
    DecodeError,
    Forbidden,
    NotFound,
    WriteFailed,
    EmptyContent,
}

#[extract_accessors]
#[derive(Clone, Debug)]
pub struct SaveFileError {
    #[access(get)]
    err_type: SaveFileErrorType,
    #[access(get_ref)]
    err_info: String,
}

#[derive(Clone, Debug)]
pub struct SaveFileResult {
    pub path: PathBuf,
}

// ---------------------------------------------------------------------------
// SaveFile implementation
// ---------------------------------------------------------------------------

impl<'a> SaveFile<'a> {
    pub fn from_json(
        projroot: &'a Path,
        filter: &'a Pathfilter,
        payload: &'a Value,
    ) -> Result<SaveFile<'a>, SaveFileError> {
        let file = match payload.get("file").and_then(|v| v.as_str()).map(PathBuf::from) {
            Some(file) => file,
            None => {
                return Err(SaveFileError {
                    err_type: SaveFileErrorType::DecodeError,
                    err_info: format!(
                        "invalid file: {}",
                        payload.get("file").unwrap_or(&Value::Null),
                    ),
                });
            }
        };
        let file = match resolve_relaxed_path(projroot, &file) {
            Some(file) => file,
            None => {
                return Err(SaveFileError::new(SaveFileErrorType::NotFound, &file, ""));
            }
        };

        let content = match get_json_field(payload, "content") {
            Ok(content) => content,
            Err(err) => {
                return Err(SaveFileError {
                    err_type: SaveFileErrorType::DecodeError,
                    err_info: format!("invalid content: {}", err),
                });
            }
        };

        let note = match payload.get("note").and_then(|v| v.as_str()) {
            Some(note) => note,
            None => {
                return Err(SaveFileError {
                    err_type: SaveFileErrorType::DecodeError,
                    err_info: format!(
                        "invalid note: {}",
                        payload.get("note").unwrap_or(&Value::Null),
                    ),
                });
            }
        };

        Ok(SaveFile {
            filter,
            path: normalize_path(&file),
            content,
            note,
        })
    }

    pub fn execute(self) -> Result<SaveFileResult, SaveFileError> {
        let path = normalize_path(&self.path);

        if !self.filter.can_write(&path) {
            return Err(SaveFileError::new(SaveFileErrorType::Forbidden, &path, ""));
        }

        if self.content.is_empty() {
            return Err(SaveFileError::new(SaveFileErrorType::EmptyContent, &path, ""));
        }

        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
        {
            Ok(mut f) => {
                if let Err(e) = f.write_all(self.content.as_bytes()) {
                    Err(SaveFileError::new(
                        SaveFileErrorType::WriteFailed,
                        &path,
                        &format!("write failed: {}", e),
                    ))
                } else {
                    Ok(SaveFileResult { path: self.path })
                }
            }

            Err(e) => {
                println!("### SAVE_FAILED: OPEN: {}", e);

                Err(SaveFileError::new(
                    SaveFileErrorType::WriteFailed,
                    &path,
                    &format!("open failed: {}", e),
                ))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SaveFileResult JSON
// ---------------------------------------------------------------------------

impl SaveFileResult {
    pub fn to_json(&self, message_id: &str) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id,
            "content": self.path.as_os_str(),
        })
    }

    pub fn to_string(&self, message_id: &str) -> String {
        format!("{:?} {}: {:?}", AIToolType::SaveFile, message_id, self.path)
    }
}

// ---------------------------------------------------------------------------
// SaveFileError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for SaveFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.err_type, self.err_info)
    }
}

impl std::error::Error for SaveFileError {}

impl SaveFileError {
    pub fn new(err_type: SaveFileErrorType, path: &Path, info: &str) -> Self {
        let err_info = format!("{}: {}", path.to_string_lossy(), info);

        Self {
            err_type,
            err_info,
        }
    }

    pub fn to_json(&'_ self, message_id: &str) -> Value {
        let msg = match self.err_type {
            SaveFileErrorType::DecodeError =>
                format!("[save_file] ERROR: decode error: {}", self.err_info),

            SaveFileErrorType::Forbidden =>
                format!("[save_file] ERROR: not allowed to read: {}", self.err_info),

            SaveFileErrorType::NotFound =>
                format!("[save_file] ERROR: file not found: {}", self.err_info),

            SaveFileErrorType::WriteFailed =>
                format!("[save_file] ERROR: write failed: {}", self.err_info),

            SaveFileErrorType::EmptyContent =>
                format!("[save_file] ERROR: empty content: {}", self.err_info),

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

impl ResultToJson for Result<SaveFileResult, SaveFileError> {
    fn to_json(&self, msg_id: &str) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<SaveFileResult, SaveFileError> {
    fn to_string(&self, msg_id: &str) -> String {
        match self {
            Ok(ok) => ok.to_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<SaveFileResult, SaveFileError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.is_ok()
    }
}

