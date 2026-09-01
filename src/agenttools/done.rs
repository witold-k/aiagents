// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use serde_json::{json, Value};
use crate::agenttools::aitooltype::{ AIToolType, ResultToString, ResultToJson, Validatable };
use crate::aimessage::AIMessageId;

#[derive(Debug)]
pub struct DoneResult {
    pub data: String,
}

// Dummy, just to keep structure intact
#[derive(Debug)]
pub struct DoneError {
}

#[derive(Debug, Default)]
pub struct Done {
    pub data: String
}

// ---------------------------------------------------------------------------
// Done
// ---------------------------------------------------------------------------

impl Done {
    pub fn from_json(payload: &Value) -> Self {
        let note = match payload.get("note") {
            Some(note) => note.to_string(),
            None       => "".to_string()
        };
        Done { data: note }
    }

    pub fn execute(self) -> Result<DoneResult, DoneError> {
        Ok(DoneResult { data: self.data })
    }
}

// ---------------------------------------------------------------------------
// DoneResult JSON
// ---------------------------------------------------------------------------

impl DoneResult {
    pub fn to_json(&self, message_id: AIMessageId) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id,
            "content": self.data,
        })
    }

    pub fn to_string(&self, message_id: AIMessageId) -> String {
        format!("{:?} {}: {}", AIToolType::Done, message_id, self.data)
    }
}

// ---------------------------------------------------------------------------
// DoneError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for DoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error occured")
    }
}

impl std::error::Error for DoneError {}

impl DoneError {
    pub fn to_json(&self, message_id: AIMessageId) -> Value {
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

impl ResultToJson for Result<DoneResult, DoneError> {
    fn to_json(&self, msg_id: AIMessageId) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<DoneResult, DoneError> {
    fn to_string(&self, msg_id: AIMessageId) -> String {
        match self {
            Ok(ok) => ok.to_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<DoneResult, DoneError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

