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

#[derive(Debug)]
pub struct ListDir<'a> {
    filter: &'a Pathfilter,
    path: PathBuf,
}

#[derive(Copy, Clone, Debug)]
pub enum ListDirErrorType {
    Forbidden,
    NotFound,
    ReadFailed,
}

#[derive(Clone, Debug)]
pub struct ListDirError {
    err_type: ListDirErrorType,
    err_info: String,
}

#[derive(Clone, Debug)]
pub struct ListDirResult {
    pub data: String,
}

// ---------------------------------------------------------------------------
// ListDir implementation
// ---------------------------------------------------------------------------

impl<'a> ListDir<'a> {
    pub fn from_json(
        projroot: &'a Path,
        filter: &'a Pathfilter,
        payload: &Value
    ) -> ListDir<'a> {
        let path = payload.get("path").and_then(|v| v.as_str()).map(PathBuf::from);
        let rpath = resolve_relaxed_path(projroot, path.unwrap());
        ListDir {
            filter,
            path: normalize_path(&rpath.unwrap()),
        }
    }

    pub fn execute(&self) -> Result<ListDirResult, ListDirError> {
        let path = &self.path;

        if !self.filter.contains(path) {
            return Err(ListDirError::new(ListDirErrorType::Forbidden, path));
        }

        if !path.exists() {
            return Err(ListDirError::new(ListDirErrorType::NotFound, path));
        }

        // Read directory entries
        let entries = match std::fs::read_dir(path) {
            Ok(read_dir) => {
                let mut list = Vec::new();
                for entry in read_dir {
                    match entry {
                        Ok(e) => list.push(e.path().display().to_string()),
                        Err(_) => {
                            return Err(ListDirError::new(ListDirErrorType::ReadFailed, path));
                        }
                    }
                }
                list
            }
            Err(_) => {
                return Err(ListDirError::new(ListDirErrorType::ReadFailed, path));
            }
        };

        Ok(ListDirResult { data: entries.join("\n") })
    }
}

// ---------------------------------------------------------------------------
// ListDirResult JSON
// ---------------------------------------------------------------------------

impl ListDirResult {
    pub fn to_json(&self, message_id: &str) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id,
            "content": self.data,
        })
    }

    pub fn to_string(&self, message_id: &str) -> String {
        format!("{}: {}", message_id, self.data)
    }
}

// ---------------------------------------------------------------------------
// ListDirError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for ListDirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.err_type, self.err_info)
    }
}

impl std::error::Error for ListDirError {}

impl ListDirError {
    pub fn new(err_type: ListDirErrorType, path: &Path) -> Self {
        Self {
            err_type,
            err_info: path.to_string_lossy().to_string(),
        }
    }

    pub fn to_json(&self, message_id: &str) -> Value {
        let msg = match self.err_type {
            ListDirErrorType::Forbidden =>
                format!("[list_dir] ERROR: not allowed to read: {}", self.err_info),

            ListDirErrorType::NotFound =>
                format!("[list_dir] ERROR: file not found: {}", self.err_info),

            ListDirErrorType::ReadFailed =>
                format!("[list_dir] ERROR: read failed: {}", self.err_info),
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

impl ResultToJson for Result<ListDirResult, ListDirError> {
    fn to_json(&self, msg_id: &str) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<ListDirResult, ListDirError> {
    fn to_string(&self, msg_id: &str) -> String {
        match self {
            Ok(ok) => ok.to_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<ListDirResult, ListDirError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.is_ok()
    }
}

