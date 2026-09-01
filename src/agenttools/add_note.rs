// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use serde_json::{json, Value};
use crate::agenttools::aitooltype::{ ResultToString, ResultToJson, Validatable };
use crate::aimessage::AIMessageId;

#[derive(Clone, Debug)]
pub struct AddNoteResult {
    pub data: String,
}

// Dummy, just to keep structure intact
#[derive(Clone, Debug)]
pub struct AddNoteError {
}

#[derive(Debug, Default)]
pub struct AddNote {
    pub data: String
}

// ---------------------------------------------------------------------------
// AddNote
// ---------------------------------------------------------------------------

impl AddNote {
    pub fn from_json(payload: &Value) -> Self {
        match payload.get("note") {
            Some(note) => AddNote { data: note.to_string() },
            None       => AddNote { data: "no note found".to_string() },
        }
    }

    pub fn execute(self) -> Result<AddNoteResult, AddNoteError>  {
        Ok(AddNoteResult { data: self.data })
    }
}

// ---------------------------------------------------------------------------
// AddNoteResult JSON
// ---------------------------------------------------------------------------

impl AddNoteResult {
    pub fn to_json(&self, message_id: AIMessageId) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id.to_string(),
            "content": self.data,
        })
    }

    pub fn to_string(&self, message_id: AIMessageId) -> String {
        format!("{}: {}", message_id, self.data)
    }
}

// ---------------------------------------------------------------------------
// AddNoteError implementation
// ---------------------------------------------------------------------------

impl fmt::Display for AddNoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error occured")
    }
}

impl std::error::Error for AddNoteError {}

impl AddNoteError {
    pub fn to_json(&self, message_id: AIMessageId) -> Value {
        json!({
            "role": "tool",
            "tool_call_id": message_id.to_string(),
            "content": "error occured",
        })
    }
}

// ---------------------------------------------------------------------------
// ResultToJson implementation
// ---------------------------------------------------------------------------

impl ResultToJson for Result<AddNoteResult, AddNoteError> {
    fn to_json(&self, msg_id: AIMessageId) -> Value {
        match self {
            Ok(ok) => ok.to_json(msg_id),
            Err(err) => err.to_json(msg_id),
        }
    }
}

impl ResultToString for Result<AddNoteResult, AddNoteError> {
    fn to_string(&self, msg_id: AIMessageId) -> String {
        match self {
            Ok(ok) => ok.to_string(msg_id),
            Err(err) => err.to_string(),
        }
    }
}

impl Validatable for Result<AddNoteResult, AddNoteError> {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.is_ok()
    }
}

