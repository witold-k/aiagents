// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use serde_json::{json, Value};
use crate::agenttools::aitooltype::{ ResultToString, ResultToJson, Validatable };
use crate::aimessage::AIMessageId;

#[derive(Clone, Debug)]
pub struct SetFocusResult {
    pub data: String,
}

// Dummy, just to keep structure intact
#[derive(Clone, Debug)]
pub struct SetFocusError {
}

#[derive(Debug, Default)]
pub struct SetFocus {
    pub data: String
}

// ---------------------------------------------------------------------------
// SetFocus
// ---------------------------------------------------------------------------

impl SetFocus {
    pub fn from_json(payload: &Value) -> Self {
        match payload.get("focus") {
            Some(note) => SetFocus { data: note.to_string() },
            None       => SetFocus { data: "no note found".to_string() },
        }
    }

    pub fn execute(self) -> Result<SetFocusResult, SetFocusError>  {
        Ok(SetFocusResult { data: self.data })
    }
}

// ---------------------------------------------------------------------------
// SetFocusResult JSON
// ---------------------------------------------------------------------------

impl SetFocusResult {
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
// SetFocusError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for SetFocusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error occured")
    }
}

impl std::error::Error for SetFocusError {}

impl SetFocusError {
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

impl ResultToJson for Result<SetFocusResult, SetFocusError> {
    fn to_json(&self, msg_id: AIMessageId) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<SetFocusResult, SetFocusError> {
    fn to_msg_string(&self, msg_id: AIMessageId) -> String {
        match self {
            Ok(ok) => ok.to_msg_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<SetFocusResult, SetFocusError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.is_ok()
    }
}

