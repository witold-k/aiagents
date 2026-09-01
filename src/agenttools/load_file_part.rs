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
pub struct LoadFilePart<'a> {
    filter: &'a Pathfilter,
    path: PathBuf,
    start: usize,
    count: usize
}

#[derive(Copy, Clone, Debug)]
pub enum LoadFilePartErrorType {
    DecodeError,
    Forbidden,
    NotFound,
    ReadFailed,
}

#[derive(Clone, Debug)]
pub struct LoadFilePartError {
    err_type: LoadFilePartErrorType,
    err_info: String,
}

#[derive(Clone, Debug)]
pub struct LoadFilePartResult {
    pub data: String,
    pub start: usize,
    pub count: usize
}

// ---------------------------------------------------------------------------
// LoadFilePart implementation
// ---------------------------------------------------------------------------

impl<'a> LoadFilePart<'a> {
    pub fn from_json(
        projroot: &'a Path,
        filter: &'a Pathfilter,
        payload: &Value,
    ) -> Result<LoadFilePart<'a>, LoadFilePartError> {
        let file = match payload.get("file").and_then(|v| v.as_str()).map(PathBuf::from) {
            Some(file) => file,
            None => {
                return Err(LoadFilePartError {
                    err_type: LoadFilePartErrorType::DecodeError,
                    err_info: format!(
                        "invalid file: {}",
                        payload.get("file").unwrap_or(&Value::Null),
                    ),
                });
            }
        };

        let start = match payload
            .get("start")
            .or_else(|| payload.get("pos"))
            .and_then(|v| v.as_i64())
        {
            Some(start) => start,
            None => {
                return Err(LoadFilePartError {
                    err_type: LoadFilePartErrorType::DecodeError,
                    err_info: format!(
                        "invalid start: {}",
                        payload
                            .get("start")
                            .or_else(|| payload.get("pos"))
                            .unwrap_or(&Value::Null),
                    ),
                });
            }
        };

        let count = match payload
            .get("count")
            .or_else(|| payload.get("len"))
            .and_then(|v| v.as_i64())
        {
            Some(count) => count,
            None => {
                return Err(LoadFilePartError {
                    err_type: LoadFilePartErrorType::DecodeError,
                    err_info: format!(
                        "invalid count: {}",
                        payload
                            .get("count")
                            .or_else(|| payload.get("len"))
                            .unwrap_or(&Value::Null),
                    ),
                });
            }
        };

        let rpath = match resolve_relaxed_path(projroot, &file) {
            Some(rpath) => rpath,
            None => {
                return Err(LoadFilePartError {
                    err_type: LoadFilePartErrorType::DecodeError,
                    err_info: format!("invalid file path: {}", file.display()),
                });
            }
        };

        Ok(LoadFilePart {
            filter,
            path: normalize_path(&rpath),
            start: (start as i32 - 1) as usize,
            count: count as usize,
        })
    }

    pub fn execute(&self) -> Result<LoadFilePartResult, LoadFilePartError> {
        let path = &self.path;

        if !self.filter.contains(path) {
            return Err(LoadFilePartError::new(LoadFilePartErrorType::Forbidden, path));
        }

        if !path.exists() {
            return Err(LoadFilePartError::new(LoadFilePartErrorType::NotFound, path));
        }

        // Read file
        let data = std::fs::read_to_string(path).map_err(|_| {
            LoadFilePartError::new(LoadFilePartErrorType::ReadFailed, path)
        })?;

        let lines_to_return = data
            .lines()
            .map(|s| s.to_owned())
            .chain(std::iter::repeat("\n".to_owned()))
            .skip(self.start)
            .take(self.count)
            .collect::<Vec<_>>().join("\n");

        Ok(LoadFilePartResult { data: lines_to_return, start: self.start, count: self.count })
    }
}

// ---------------------------------------------------------------------------
// LoadFilePartResult JSON
// ---------------------------------------------------------------------------

impl LoadFilePartResult {
    pub fn to_json(&self, message_id: AIMessageId) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id,
            "content": self.data,
        })
    }

    pub fn to_string(&self, message_id: AIMessageId) -> String {
        format!("{}: {}", message_id, self.data)
    }
}

// ---------------------------------------------------------------------------
// LoadFilePartError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for LoadFilePartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.err_type, self.err_info)
    }
}

impl std::error::Error for LoadFilePartError {}

impl LoadFilePartError {
    pub fn new(err_type: LoadFilePartErrorType, path: &Path) -> Self {
        Self {
            err_type,
            err_info: path.to_string_lossy().to_string(),
        }
    }

    pub fn to_json(&self, message_id: AIMessageId) -> Value {
        let msg = match self.err_type {
            LoadFilePartErrorType::DecodeError =>
                format!("[load_file_part] ERROR: decode error: {}", self.err_info),

            LoadFilePartErrorType::Forbidden =>
                format!("[load_file_part] ERROR: not allowed to read: {}", self.err_info),

            LoadFilePartErrorType::NotFound =>
                format!("[load_file_part] ERROR: file not found: {}", self.err_info),

            LoadFilePartErrorType::ReadFailed =>
                format!("[load_file_part] ERROR: read failed: {}", self.err_info),
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

impl ResultToJson for Result<LoadFilePartResult, LoadFilePartError> {
    fn to_json(&self, msg_id: AIMessageId) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<LoadFilePartResult, LoadFilePartError> {
    fn to_string(&self, msg_id: AIMessageId) -> String {
        match self {
            Ok(ok) => ok.to_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<LoadFilePartResult, LoadFilePartError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.is_ok()
    }
}

