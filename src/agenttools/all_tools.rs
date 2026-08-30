// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::Path;
use serde_json::Value;
//use struct_extractors::same_entries;
use fsscanner::pathfilter::Pathfilter;
use crate::agenttools::{
    aitooltype::{AIToolType, TOOLS, ResultToString, ResultToJson, Validatable},
    ast::*,
    add_note::*,
    done::*,
    failed::*,
    list_dir::*,
    load_file::*,
    load_file_part::*,
    save_file::*,
    save_file_replace_part::*,
    scan_dir::*,
    set_focus::*,
    valid::*,
};
//use crate::agenttools::aitooltype::__BASE_ENTRIES_AIToolType;

#[derive(Debug)]
//#[same_entries(AIToolType, ToolOutput)]
pub enum ToolOutput {
    AddNote(Result<AddNoteResult, AddNoteError>),
    Ast(Result<AstResult, AstError>),
    Done(Result<DoneResult, DoneError>),
    Failed(Result<FailedResult, FailedError>),
    ListDir(Result<ListDirResult, ListDirError>),
    LoadFile(Result<LoadFileResult, LoadFileError>),
    LoadFilePart(Result<LoadFilePartResult, LoadFilePartError>),
    SaveFile(Result<SaveFileResult, SaveFileError>),
    SaveFilePart(Result<SaveFilePartResult, SaveFilePartError>),
    ScanDir(Result<ScanDirResult, ScanDirError>),
    SetFocus(Result<SetFocusResult, SetFocusError>),
    Valid(Result<ValidResult, ValidError>),
}

// #[enum_delegate(is_valid(&self) -> bool, to_json(&self, message_id: &str) -> Value)]
impl ToolOutput {
    pub fn is_valid(&self) -> bool {
        match self {
            ToolOutput::AddNote(r)  => r.is_valid(),
            ToolOutput::Ast(r)      => r.is_valid(),
            ToolOutput::Done(r)     => r.is_valid(),
            ToolOutput::Failed(r)   => r.is_valid(),
            ToolOutput::ListDir(r)  => r.is_valid(),
            ToolOutput::LoadFile(r) => r.is_valid(),
            ToolOutput::LoadFilePart(r) => r.is_valid(),
            ToolOutput::SaveFile(r) => r.is_valid(),
            ToolOutput::SaveFilePart(r) => r.is_valid(),
            ToolOutput::ScanDir(r)  => r.is_valid(),
            ToolOutput::SetFocus(r) => r.is_valid(),
            ToolOutput::Valid(r)    => r.is_valid(),
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self, ToolOutput::Done(_))
    }

    pub fn to_base(&self) -> AIToolType {
        match self {
            ToolOutput::AddNote(_)  => AIToolType::AddNote,
            ToolOutput::Ast(_)      => AIToolType::Ast,
            ToolOutput::Done(_)     => AIToolType::Done,
            ToolOutput::Failed(_)   => AIToolType::Failed,
            ToolOutput::ListDir(_)  => AIToolType::ListDir,
            ToolOutput::LoadFile(_) => AIToolType::LoadFile,
            ToolOutput::LoadFilePart(_) => AIToolType::LoadFilePart,
            ToolOutput::SaveFile(_) => AIToolType::SaveFile,
            ToolOutput::SaveFilePart(_) => AIToolType::SaveFilePart,
            ToolOutput::ScanDir(_)  => AIToolType::ScanDir,
            ToolOutput::SetFocus(_) => AIToolType::SetFocus,
            ToolOutput::Valid(_)    => AIToolType::Valid,
        }
    }

    pub fn to_json(&self, message_id: &str) -> Value {
        match self {
            ToolOutput::AddNote(r)  => r.to_json(message_id),
            ToolOutput::Ast(r)      => r.to_json(message_id),
            ToolOutput::Done(r)     => r.to_json(message_id),
            ToolOutput::Failed(r)   => r.to_json(message_id),
            ToolOutput::ListDir(r)  => r.to_json(message_id),
            ToolOutput::LoadFile(r) => r.to_json(message_id),
            ToolOutput::LoadFilePart(r) => r.to_json(message_id),
            ToolOutput::SaveFile(r) => r.to_json(message_id),
            ToolOutput::SaveFilePart(r) => r.to_json(message_id),
            ToolOutput::ScanDir(r)  => r.to_json(message_id),
            ToolOutput::SetFocus(r) => r.to_json(message_id),
            ToolOutput::Valid(r)    => r.to_json(message_id),
        }
    }

    pub fn to_string(&self, message_id: &str) -> String {
        match self {
            ToolOutput::AddNote(r)  => r.to_string(message_id),
            ToolOutput::Ast(r)      => r.to_string(message_id),
            ToolOutput::Done(r)     => r.to_string(message_id),
            ToolOutput::Failed(r)   => r.to_string(message_id),
            ToolOutput::ListDir(r)  => r.to_string(message_id),
            ToolOutput::LoadFile(r) => r.to_string(message_id),
            ToolOutput::LoadFilePart(r) => r.to_string(message_id),
            ToolOutput::SaveFile(r) => r.to_string(message_id),
            ToolOutput::SaveFilePart(r) => r.to_string(message_id),
            ToolOutput::ScanDir(r)  => r.to_string(message_id),
            ToolOutput::SetFocus(r) => r.to_string(message_id),
            ToolOutput::Valid(r)    => r.to_string(message_id),
        }
    }
}

pub fn execute_tool<'a>(
    payload: &'a Value, projroot: &'a Path, filter: &'a Pathfilter,
) -> ToolOutput {

    let action = match payload.get("action").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => return ToolOutput::Failed(Failed::from_string("No action found in payload".to_string()).execute()),
    };
    let action_key = action.to_string();

    match TOOLS.get(&action_key) {
        Some(AIToolType::AddNote)  => ToolOutput::AddNote(AddNote::from_json(payload).execute()),
        Some(AIToolType::Ast)      => ToolOutput::Ast(Ast::from_json(filter, payload).execute()),
        Some(AIToolType::Done)     => ToolOutput::Done(Done::default().execute()),
        Some(AIToolType::Failed)   => ToolOutput::Failed(Failed::default().execute()),
        Some(AIToolType::ListDir)  => {
            match ListDir::from_json(projroot, filter, payload) {
                Ok(obj)  => ToolOutput::ListDir(obj.execute()),
                Err(err) => ToolOutput::Failed(Failed::from_string(format!("{}", err)).execute())
            }
        },
        Some(AIToolType::LoadFile)  => {
            match LoadFile::from_json(projroot, filter, payload) {
                Ok(obj)  => ToolOutput::LoadFile(obj.execute()),
                Err(err) => ToolOutput::Failed(Failed::from_string(format!("{}", err)).execute())
            }
        },
        Some(AIToolType::LoadFilePart)  => {
            match LoadFilePart::from_json(projroot, filter, payload) {
                Ok(obj)  => ToolOutput::LoadFilePart(obj.execute()),
                Err(err) => ToolOutput::Failed(Failed::from_string(format!("{}", err)).execute())
            }
        },
        Some(AIToolType::SaveFile)  => {
            match SaveFile::from_json(projroot, filter, payload) {
                Ok(obj)  => ToolOutput::SaveFile(obj.execute()),
                Err(err) => ToolOutput::Failed(Failed::from_string(format!("{}", err)).execute())
            }
        },
        Some(AIToolType::SaveFilePart)  => {
            match SaveFilePart::from_json(projroot, filter, payload) {
                Ok(obj)  => ToolOutput::SaveFilePart(obj.execute()),
                Err(err) => ToolOutput::Failed(Failed::from_string(format!("{}", err)).execute())
            }
        },
        Some(AIToolType::ScanDir)  => {
            match ScanDir::from_json(projroot, filter, payload) {
                Ok(obj)  => ToolOutput::ScanDir(obj.execute()),
                Err(err) => ToolOutput::Failed(Failed::from_string(format!("{}", err)).execute())
            }
        },
        Some(AIToolType::SetFocus) => ToolOutput::SetFocus(SetFocus::from_json(payload).execute()),
        Some(AIToolType::Valid)    => ToolOutput::Valid(Valid::default().execute()),
        None => ToolOutput::Failed(Failed::from_string(format!("key not found: {}", payload)).execute())
    }
}

