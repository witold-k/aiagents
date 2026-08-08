// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use serde_json::{json, Value};
use crate::agenttools::aitooltype::{ ResultToJson, ResultToString, Validatable };

#[derive(Clone, Debug)]
pub struct ValidResult {
    pub data: String,
}

// Dummy, just to keep structure intact
#[derive(Clone, Debug)]
pub struct ValidError {
}

#[derive(Debug, Default)]
pub struct Valid {
    pub data: String
}

// ---------------------------------------------------------------------------
// Valid
// ---------------------------------------------------------------------------

impl Valid {
    pub fn from_string(data: String) -> Self {
        Valid { data }
    }

    pub fn execute(self) -> Result<ValidResult, ValidError> {
        Ok(ValidResult { data: self.data })
    }
}

// ---------------------------------------------------------------------------
// ValidResult JSON
// ---------------------------------------------------------------------------

impl ValidResult {
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
// ValidError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for ValidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error occured")
    }
}

impl std::error::Error for ValidError {}

impl ValidError {
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

impl ResultToJson for Result<ValidResult, ValidError> {
    fn to_json(&self, msg_id: &str) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<ValidResult, ValidError> {
    fn to_string(&self, msg_id: &str) -> String {
        match self {
            Ok(ok) => ok.to_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<ValidResult, ValidError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

