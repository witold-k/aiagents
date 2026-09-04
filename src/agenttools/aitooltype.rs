// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde::{Serialize, Deserialize};
use serde_json::Value;
use phf::phf_map;
use struct_extractors::base_entries;
use crate::aimessage::AIMessageId;

#[base_entries(AIToolType)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AIToolType {
    AddNote,
    Ast,
    Done,
    Failed,
    ListDir,
    LoadFile,
    LoadFilePart,
    SaveFile,
    SaveFilePart,
    ScanDir,
    SetFocus,
    Valid,
}

pub trait ResultToJson {
    fn to_json(&self, msg_id: AIMessageId) -> Value;
}

pub trait ResultToString {
    fn to_msg_string(&self, msg_id: AIMessageId) -> String;
}

pub trait Validatable {
    fn is_valid(&self) -> bool;
}

impl AIToolType {
    pub fn is_save(self) -> bool {
        matches!(self, AIToolType::SaveFile | AIToolType::SaveFilePart)
    }

    pub fn is_ast(self) -> bool {
        matches!(self, AIToolType::Ast)
    }

    pub fn is_load(self) -> bool {
        matches!(self, AIToolType::LoadFile | AIToolType::LoadFilePart)
    }

    pub fn is_done(self) -> bool {
        matches!(self, AIToolType::Done)
    }

    pub fn is_failed(self) -> bool {
        matches!(self, AIToolType::Failed)
    }

    pub fn is_note(self) -> bool {
        matches!(self, AIToolType::AddNote)
    }

    pub fn is_full_load(self) -> bool {
        self == AIToolType::LoadFile
    }
}

pub static TOOLS: phf::Map<&'static str, AIToolType> = phf_map! {
    "add_note" => AIToolType::AddNote,
    "ast" => AIToolType::Ast,
    "done" => AIToolType::Done,
    "failed" => AIToolType::Failed,
    "list_dir" => AIToolType::ListDir,
    "load_file" => AIToolType::LoadFile,
    "load_file_part" => AIToolType::LoadFilePart,
    "save_file" => AIToolType::SaveFile,
    "save_file_part" => AIToolType::SaveFilePart,
    "set_focus" => AIToolType::SetFocus,
    "valid" => AIToolType::Valid,
};

