// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//!
//! just a placeholder for further functionaliy
//! for now the same as ListDir
//!

use std::fmt;
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use fsscanner::{
    pathfilter::Pathfilter,
    pathutils::{normalize_path, resolve_relaxed_path},
};
use crate::agenttools::aitooltype::{ ResultToString, ResultToJson, Validatable };

#[derive(Debug)]
pub struct ScanDir<'a> {
    filter: &'a Pathfilter,
    path: PathBuf,
}

#[derive(Copy, Clone, Debug)]
pub enum ScanDirErrorType {
    Forbidden,
    NotFound,
    ReadFailed,
}

#[derive(Clone, Debug)]
pub struct ScanDirError {
    err_type: ScanDirErrorType,
    err_info: String,
}

#[derive(Clone, Debug)]
pub struct ScanDirResult {
    pub data: String,
}

// ---------------------------------------------------------------------------
// ScanDir implementation
// ---------------------------------------------------------------------------

impl<'a> ScanDir<'a> {
    pub fn from_json(
        projroot: &'a Path,
        filter: &'a Pathfilter,
        payload: &Value
    ) -> ScanDir<'a> {
        let path = payload.get("path").and_then(|v| v.as_str()).map(PathBuf::from);
        let rpath = resolve_relaxed_path(projroot, path.unwrap());
        ScanDir {
            filter,
            path: normalize_path(&rpath.unwrap()),
        }
    }

    pub fn execute(&self) -> Result<ScanDirResult, ScanDirError> {
        let path = &self.path;

        if !self.filter.contains(path) {
            return Err(ScanDirError::new(ScanDirErrorType::Forbidden, path));
        }

        if !path.exists() {
            return Err(ScanDirError::new(ScanDirErrorType::NotFound, path));
        }

        // Read directory entries
        let entries = match std::fs::read_dir(path) {
            Ok(read_dir) => {
                let mut list = Vec::new();
                for entry in read_dir {
                    match entry {
                        Ok(e) => list.push(e.path().display().to_string()),
                        Err(_) => {
                            return Err(ScanDirError::new(ScanDirErrorType::ReadFailed, path));
                        }
                    }
                }
                list
            }
            Err(_) => {
                return Err(ScanDirError::new(ScanDirErrorType::ReadFailed, path));
            }
        };

        Ok(ScanDirResult { data: entries.join("\n") })
    }
}

// ---------------------------------------------------------------------------
// ScanDirResult JSON
// ---------------------------------------------------------------------------

impl ScanDirResult {
    pub fn to_json(&self, message_id: &str) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id,
            "content": self.data,
        })
    }

    pub fn to_string(&self, message_id: &str) -> String {
        format!("{}: {:?}", message_id, self.data)
    }
}

// ---------------------------------------------------------------------------
// ScanDirError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for ScanDirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.err_type, self.err_info)
    }
}

impl std::error::Error for ScanDirError {}

impl ScanDirError {
    pub fn new(err_type: ScanDirErrorType, path: &Path) -> Self {
        Self {
            err_type,
            err_info: path.to_string_lossy().to_string(),
        }
    }

    pub fn to_json(&self, message_id: &str) -> Value {
        let msg = match self.err_type {
            ScanDirErrorType::Forbidden =>
                format!("[list_dir] ERROR: not allowed to read: {}", self.err_info),

            ScanDirErrorType::NotFound =>
                format!("[list_dir] ERROR: file not found: {}", self.err_info),

            ScanDirErrorType::ReadFailed =>
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

impl ResultToJson for Result<ScanDirResult, ScanDirError> {
    fn to_json(&self, msg_id: &str) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<ScanDirResult, ScanDirError> {
    fn to_string(&self, msg_id: &str) -> String {
        match self {
            Ok(ok) => ok.to_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<ScanDirResult, ScanDirError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.is_ok()
    }
}

