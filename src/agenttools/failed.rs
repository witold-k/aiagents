// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use serde_json::{json, Value};
use crate::agenttools::aitooltype::{ ResultToString, ResultToJson, Validatable };

#[derive(Debug)]
pub struct FailedResult {
    pub data: String,
}

// Dummy, just to keep structure intact
#[derive(Clone, Debug)]
pub struct FailedError {
}

#[derive(Clone, Debug, Default)]
pub struct Failed {
    pub data: String
}

// ---------------------------------------------------------------------------
// Failed
// ---------------------------------------------------------------------------

impl Failed {
    pub fn from_string(data: String) -> Self {
        Failed { data }
    }

    pub fn from_json(payload: &Value) -> Self {
        let note = match payload.get("failed") {
            Some(note) => note.to_string(),
            None       => "".to_string()
        };
        Failed { data: note }
    }

    pub fn execute(self) -> Result<FailedResult, FailedError> {
        Ok(FailedResult { data: self.data })
    }
}

// ---------------------------------------------------------------------------
// FailedResult JSON
// ---------------------------------------------------------------------------

impl FailedResult {
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
// FailedError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for FailedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error occured")
    }
}

impl std::error::Error for FailedError {}

impl FailedError {
    pub fn to_json(&self, message_id: &str) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id,
            "content": "error occured",
        })
    }
}

// ---------------------------------------------------------------------------
// ResultToJson implementation
// ---------------------------------------------------------------------------

impl ResultToJson for Result<FailedResult, FailedError> {
    fn to_json(&self, msg_id: &str) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<FailedResult, FailedError> {
    fn to_string(&self, msg_id: &str) -> String {
        match self {
            Ok(ok) => ok.to_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<FailedResult, FailedError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        false
    }
}

