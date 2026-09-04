// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use fsscanner::{
    pathfilter::Pathfilter,
    pathutils::{normalize_path, resolve_relaxed_path},
};
use crate::agenttools::aitooltype::{ ResultToString, ResultToJson, Validatable };
use crate::aimessage::AIMessageId;

#[derive(Debug)]
pub struct LoadFile<'a> {
    filter: &'a Pathfilter,
    path: PathBuf,
}

#[derive(Copy, Clone, Debug)]
pub enum LoadFileErrorType {
    DecodeError,
    Forbidden,
    NotFound,
    ReadFailed,
}

#[derive(Clone, Debug)]
pub struct LoadFileError {
    err_type: LoadFileErrorType,
    err_info: String,
}

#[derive(Clone, Debug)]
pub struct LoadFileResult {
    pub data: String,
}

// ---------------------------------------------------------------------------
// LoadFile implementation
// ---------------------------------------------------------------------------

impl<'a> LoadFile<'a> {
    pub fn from_json(
        projroot: &'a Path,
        filter: &'a Pathfilter,
        payload: &Value,
    ) -> Result<LoadFile<'a>, LoadFileError> {
        match payload.get("file").and_then(|v| v.as_str()).map(PathBuf::from) {
            Some(file) => {
                match resolve_relaxed_path(projroot, &file) {
                    Some(rpath) => Ok(LoadFile {
                        filter,
                        path: normalize_path(&rpath),
                    }),
                    None => Err(LoadFileError::new(LoadFileErrorType::NotFound, &file))
                }
            },
            None => {
                Err(LoadFileError {
                    err_type: LoadFileErrorType::DecodeError,
                    err_info: "file".to_string(),
                })
            },
        }
    }

    pub fn execute(&self) -> Result<LoadFileResult, LoadFileError> {
        let path = &self.path;

        if !self.filter.contains(path) {
            return Err(LoadFileError::new(LoadFileErrorType::Forbidden, path));
        }

        if !path.exists() {
            return Err(LoadFileError::new(LoadFileErrorType::NotFound, path));
        }

        let data = std::fs::read_to_string(path).map_err(|e| {
            println!("[load_file] ERROR: {:?}", e);
            LoadFileError::new(LoadFileErrorType::ReadFailed, path)
        })?;

        Ok(LoadFileResult { data })
    }
}

// ---------------------------------------------------------------------------
// LoadFileResult JSON
// ---------------------------------------------------------------------------

impl LoadFileResult {
    pub fn to_json(&self, message_id: AIMessageId) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id,
            "content": self.data,
        })
    }

    pub fn to_msg_string(&self, message_id: AIMessageId) -> String {
        format!("{}: {}", message_id, self.data)
    }
}

// ---------------------------------------------------------------------------
// LoadFileError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for LoadFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.err_type, self.err_info)
    }
}

impl std::error::Error for LoadFileError {}

impl LoadFileError {
    pub fn new(err_type: LoadFileErrorType, path: &Path) -> Self {
        Self {
            err_type,
            err_info: path.to_string_lossy().to_string(),
        }
    }

    pub fn to_json(&self, message_id: AIMessageId) -> Value {
        let msg = match self.err_type {
            LoadFileErrorType::DecodeError =>
                format!("[load_file] ERROR: decode error: {}", self.err_info),

            LoadFileErrorType::Forbidden =>
                format!("[load_file] ERROR: not allowed to read: {}", self.err_info),

            LoadFileErrorType::NotFound =>
                format!("[load_file] ERROR: file not found: {}", self.err_info),

            LoadFileErrorType::ReadFailed =>
                format!("[load_file] ERROR: read failed: {}", self.err_info),
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

impl ResultToJson for Result<LoadFileResult, LoadFileError> {
    fn to_json(&self, msg_id: AIMessageId) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<LoadFileResult, LoadFileError> {
    fn to_msg_string(&self, msg_id: AIMessageId) -> String {
        match self {
            Ok(ok) => ok.to_msg_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<LoadFileResult, LoadFileError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.is_ok()
    }
}

