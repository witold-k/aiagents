// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use fsscanner::fileentry::FileEntry;
use std::fmt::{Display, Formatter};
use crate::agenttools::aitooltype::AIToolType;
use crate::generated_tasks::Tasks;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AIMessageType {
    System,
    User,
    Build,
    Tool,
    Model,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AIMessageId {
    pub val: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessage {
    pub message_id: AIMessageId,
    pub msgtype:    AIMessageType,
    pub tooltype:   AIToolType, // in case AIMessageType::Tool
    pub data:       String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIFileinfo {
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessageList {
    pub messages: Vec<AIMessage>,
    pub message_id: AIMessageId,
    pub depth:      usize,
    pub task_id:    Tasks,
    pub task_description: String,
    pub subtask:       Vec<String>,
    pub structureinfo: String,
    pub files:  Vec<FileEntry>,
    pub note:   String,
    pub focus:  String,
    pub faults: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessageListData {
    pub messages:   Vec<AIMessage>,
    pub message_id: AIMessageId,
    pub depth:      usize,
    pub task_id:    Tasks,
    pub task_description: String,
    pub subtask:          Vec<String>,
    pub structureinfo:    String,
    pub files:  Vec<FileEntry>,
    pub focus:  String,
    pub faults: Option<String>,
}

//
// ---------------------------------------------------------------------------
//

impl AIMessageType {
    #[inline(always)]
    pub fn to_str(&self) -> &str {
        match self {
            AIMessageType::System => "system",
            AIMessageType::User   => "user",
            AIMessageType::Build  => "build",
            AIMessageType::Tool   => "tool",
            AIMessageType::Model  => "assistant",
        }
    }
}

impl Display for AIMessageId {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "call_{}", self.val)
    }
}

impl AIMessage {
    #[inline(always)]
    pub fn new(
        message_id: AIMessageId,
        msgtype:    AIMessageType,
        tooltype:   AIToolType,
        data:       String
    ) -> Self {
        Self { message_id, msgtype, tooltype, data }
    }

    #[inline(always)]
    pub fn is_user(&self) -> bool {
        AIMessageType::User == self.msgtype
    }

    pub fn to_json(&self) -> Value {
        json!({
            "role":         self.msgtype.to_str(),
            "tool_call_id": &self.message_id.to_string(),
            "content":      &self.data
        })
    }
}

impl AIMessageList {
    pub fn new(data: AIMessageListData) -> Self {
        Self {
            messages:   data.messages,
            message_id: data.message_id,
            depth:      data.depth,
            task_id:    data.task_id,
            task_description: data.task_description,
            subtask:       data.subtask,
            structureinfo: data.structureinfo,
            files:         data.files,
            note:       "".into(),
            focus:      data.focus,
            faults:     data.faults,
        }
    }

    pub fn inc_messageid(&mut self) -> AIMessageId {
        self.message_id.val += 1;
        self.message_id
    }

    pub fn append(
        &mut self,
        message_id: AIMessageId,
        msgtype: AIMessageType,
        tooltype: AIToolType,
        data: &str
    ) {
        self.messages.push(AIMessage::new(message_id, msgtype, tooltype, data.to_string()));
        if (AIToolType::LoadFile == tooltype) || (AIToolType::Ast == tooltype) {
            let filename      = data;
            let mut req_found = false;
            for message in &mut self.messages {
                if
                    (AIMessageType::Model == message.msgtype) &&
                    (tooltype == message.tooltype)
                {
                    let content = &message.data;
                    if content == filename {
                        req_found = true;
                    }
                }
                else {
                    if req_found {
                        message.data = ["REF: ", &message_id.to_string()].concat();
                        req_found = false;
                    }
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.faults = None;
        self.messages.clear();
    }

    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::{json, Value};

        let mut arr = Vec::<Value>::new();
        let mut content = String::with_capacity(10 * 1024);

        content.push_str(&self.task_description);
        content.push('\n');

        for entry in &self.subtask {
            content.push_str(entry);
            content.push('\n');
        }

        if !self.structureinfo.is_empty() {
            let fdata = format!("=== INFO/AST ===\n{}", self.structureinfo);
            content.push_str(&fdata);
            content.push('\n');
        }

        if !self.files.is_empty() {
            let fdata = format!(
                "=== FILES ===\n{}",
                self.files
                    .iter()
                    .map(|f| f.path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            println!("{}", fdata);

            let fdata = format!(
                "=== FILES ===\n{}",
                self.files
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            content.push_str(&fdata);
            content.push('\n');
        }

        if let Some(ref faults) = self.faults {
            content.push_str(faults);
            content.push('\n');
        }

        if !self.note.is_empty() {
            let fdata = format!("=== NOTE ===\n{}", self.note);
            println!("{}", fdata);
            content.push_str(&fdata);
            content.push('\n');
        }

        if !self.focus.is_empty() {
            let fdata = format!("=== FOCUS ===\n{}", self.focus);
            content.push_str(&fdata);
            content.push('\n');
        }

        let prepend_user = self.messages.first().is_some_and(|m| m.is_user());
        if prepend_user {
            let first = &self.messages[0];
            content.push_str(&first.data);
        }
        else {
            arr.push(json!({
                "role": "user",
                "content": content
            }));
        }
        for entry in &self.messages {
            arr.push(entry.to_json());
        }

        Value::Array(arr)
    }

    pub fn cut_to_depth(&mut self) {
        if self.messages.len() > self.depth {
            let start = self.messages.len() - self.depth;
            self.messages = self.messages[start..].to_vec();
        }
    }

    pub fn remove_type(&mut self, t: AIMessageType) {
        self.messages.retain(|m| m.msgtype != t);
    }
}

