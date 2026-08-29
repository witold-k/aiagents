// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde::{Serialize, Deserialize};
use serde_json::Value;
use fsscanner::fileentry::FileEntry;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessage {
    pub msgtype:  AIMessageType,
    pub tooltype: AIToolType, // in case AIMessageType::Tool
    pub data:     Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIFileinfo {
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessageList {
    pub messages: Vec<AIMessage>,
    pub message_id: usize,
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
    pub message_id: usize,
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

impl AIMessage {
    pub fn new(msgtype: AIMessageType, tooltype: AIToolType, data: Value) -> Self {
        Self { msgtype, tooltype, data }
    }
}

impl AIMessageList {
    pub fn new(data: AIMessageListData) -> Self {
        Self {
            messages:   data.messages,
            message_id: data.message_id,
            depth:   data.depth,
            task_id: data.task_id,
            task_description: data.task_description,
            subtask:       data.subtask,
            structureinfo: data.structureinfo,
            files:         data.files,
            note: "".into(),
            focus:  data.focus,
            faults: data.faults,
        }
    }

    pub fn inc_messageid(&mut self) -> usize {
        self.message_id += 1;
        self.message_id
    }

    pub fn append(&mut self, message_id: &str, msgtype: AIMessageType, tooltype: AIToolType, mut data: Value) {
        data["tool_call_id"] = message_id.into();
        self.messages.push(AIMessage::new(msgtype, tooltype, data.clone()));
        if (AIToolType::LoadFile == tooltype) || (AIToolType::Ast == tooltype) {
            let filename = match data.get("content") {
                Some(filename) => filename.to_string(),
                None => return,
            };
            let mut req_found = false;
            for message in &mut self.messages {
                if
                    (AIMessageType::Model == message.msgtype) &&
                    (tooltype == message.tooltype)
                {
                    let content = match message.data.get("content") {
                        Some(content) => content,
                        None          => continue,
                    };
                    if *content.to_string() == filename {
                        req_found = true;
                    }
                }
                else {
                    if req_found {
                        message.data["content"] = serde_json::Value::String(
                            ["REF: ", message_id].concat()
                        );
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

        // Helper to push a simple {role, content} message
        let mut push_msg = |role: &str, content: &str| {
            arr.push(json!({
                "role": role,
                "content": content
            }));
        };

        push_msg("user", &self.task_description);

        for entry in &self.subtask {
            push_msg("user", entry);
        }

        if !self.structureinfo.is_empty() {
            let fdata = format!("=== INFO/AST ===\n{}", self.structureinfo);
            push_msg("user", &fdata);
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
            push_msg("user", &fdata);
        }

        if let Some(ref faults) = self.faults {
            push_msg("user", faults);
        }

        if !self.note.is_empty() {
            let fdata = format!("=== NOTE ===\n{}", self.note);
            println!("{}", fdata);
            push_msg("user", &fdata);
        }

        if !self.focus.is_empty() {
            let fdata = format!("=== FOCUS ===\n{}", self.focus);
            push_msg("user", &fdata);
        }

        // Messages already contain JSON objects → push them directly
        for entry in &self.messages {
            arr.push(entry.data.clone());
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

    pub fn set_last_callid(&mut self, message_id: &str) {
        let l = self.messages.len();
        if l != 0 {
            self.messages[l - 1].data["tool_call_id"] = serde_json::json!(message_id);
        }
    }
}

