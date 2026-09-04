// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use fsscanner::pathfilter::Pathfilter;
use crate::agenttools::aitooltype::{ ResultToString, ResultToJson, Validatable };
use crate::aimessage::AIMessageId;

#[derive(Clone, Debug)]
pub struct AstError {
    err_type: AstErrorType,
    err_info: String,
}

#[derive(Clone, Debug)]
pub struct AstResult {
    pub data: String,
}

#[derive(Debug)]
pub struct Ast<'a> {
    filter: &'a Pathfilter,
    path: PathBuf,
}

#[derive(Copy, Clone, Debug)]
pub enum AstErrorType {
    Forbidden,
    NotFound,
    ExeFailed,
}

// ---------------------------------------------------------------------------
// Ast implementation
// ---------------------------------------------------------------------------

impl<'a> Ast<'a> {
    pub fn from_json(filter: &'a Pathfilter, payload: &Value) -> Ast<'a> {
        match payload.get("path").and_then(|v| v.as_str()).map(PathBuf::from) {
            Some(path) => Ast { filter, path },
            None       => Ast { filter, path: PathBuf::from("") },
        }
    }

    pub fn execute(&self) -> Result<AstResult, AstError> {
        let path = &self.path;

        if !self.filter.contains(path) {
            return Err(AstError::new(AstErrorType::Forbidden, path));
        }

        if !path.exists() {
            return Err(AstError::new(AstErrorType::NotFound, path));
        }

        match std::process::Command::new("ast-outline")
            .arg(path)
            .output()
        {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                Ok(AstResult { data: stdout })
            }
            Err(_) => {
                Err(AstError::new(AstErrorType::ExeFailed, path))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AstResult JSON
// ---------------------------------------------------------------------------

impl AstResult {
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
// AstError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for AstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.err_type, self.err_info)
    }
}

impl std::error::Error for AstError {}

impl AstError {
    pub fn new(err_type: AstErrorType, path: &Path) -> Self {
        Self {
            err_type,
            err_info: path.to_string_lossy().to_string(),
        }
    }

    pub fn to_json(&self, message_id: AIMessageId) -> Value {
        let msg = match self.err_type {
            AstErrorType::Forbidden =>
                format!("[ast] ERROR: not allowed to read: {}", self.err_info),

            AstErrorType::NotFound =>
                format!("[ast] ERROR: file not found: {}", self.err_info),

            AstErrorType::ExeFailed =>
                format!("[ast] ERROR: execute failed: {}", self.err_info),
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

impl ResultToJson for Result<AstResult, AstError> {
    fn to_json(&self, msg_id: AIMessageId) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<AstResult, AstError> {
    fn to_msg_string(&self, msg_id: AIMessageId) -> String {
        match self {
            Ok(ok) => ok.to_msg_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<AstResult, AstError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.is_ok()
    }
}
