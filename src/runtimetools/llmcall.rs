// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde_json::Value;
use std::cell::RefCell;
use std::fmt;
use std::path::{Path, PathBuf};
use fsscanner::{
    fileentry::FileEntry,
    fsscanner_base::collect_files_all,
    pathfilter::Pathfilter,
    pathutils::normalize_path,
};
use crate::agenttools::{
    all_tools::{execute_tool, ToolOutput},
    aitooltype::AIToolType,
    failed::*,
};
use crate::aimessageid::AIMessageId;
use crate::config::AIProvider;
use crate::runtimetools::aimessage::{AIMessageList, AIMessageType, AIMessageListData};
use crate::runtimetools::airequest::{AIRequest, AIRequestResult};
use crate::utils:: {
    ast::get_ast_string,
    scan_dir::scan_with_suffix_and_filter,
    stringutils::{strip_code_fences, raw_fence_to_string},
};
use crate::config::Config;
use crate::generated_tasks::Tasks;

#[expect(dead_code)]
#[derive(Clone)]
pub struct LlmCall<'a> {
    config: &'a Config,
    provider: &'a AIProvider,
    projdir: PathBuf,
    workspacedir: Option<PathBuf>,
    filter: &'a Pathfilter,
    messages: RefCell<AIMessageList>,
    dump: bool,
}

pub enum LlmCallResult {
    Ok,
    Done,
    RetryFailed,
    ToolResult(ToolOutput),
    RequestError(AIRequestResult),
    ToolError(ToolOutput)
}

impl LlmCallResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Ok | Self::Done | Self::ToolResult(_))
    }

    pub fn is_request_error(&self) -> bool {
        matches!(self, Self::RequestError(_))
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}

impl fmt::Display for LlmCallResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "Ok"),
            Self::Done => write!(f, "Done"),
            Self::RetryFailed => write!(f, "RetryFailed"),
            Self::ToolResult(result) => write!(f, "{result}"),
            Self::RequestError(result) => write!(f, "{result}"),
            Self::ToolError(result) => write!(f, "{result}"),
        }
    }
}

impl<'a> LlmCall<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: &'a Config,
        provider: &'a AIProvider,
        projdir: PathBuf,
        workspacedir: Option<PathBuf>,
        task_id: Tasks,
        task_description: String,
        subtask: Vec<String>,
        filter: &'a Pathfilter,
        selected: &'a [PathBuf],
        dump: bool,
    ) -> Self {
        let data = AIMessageListData {
            messages: Vec::new(),
            message_id: AIMessageId {val: 1},
            depth: config.queue_length_max,
            task_id,
            task_description,
            subtask,
            structureinfo: Self::create_structure_info(),
            context: String::new(),
            files: Self::create_files_info(
                config, &projdir, workspacedir.as_deref(), task_id, filter, selected
            ).unwrap_or_else(|err| {
                eprintln!("Failed to load initial file context: {err}");
                Vec::new()
            }),
            focus: "".into(),
            faults: None,
        };
        Self {
            config,
            provider,
            projdir: normalize_path(&projdir),
            workspacedir,
            filter,
            messages: RefCell::new(AIMessageList::new(data)),
            dump,
        }
    }

    // FIXME should be moved to utils and used in workflow (eg. BLT)
    pub fn create_structure_info() -> String {
        get_ast_string("src")
    }

    // FIXME should be moved to utils and used in workflow (eg. BLT)
    pub fn create_files_info(
        config: &Config,
        projdir: &Path,
        workspacedir: Option<&Path>,
        _task_id: Tasks,
        filter: &'a Pathfilter,
        selected: &'a [PathBuf],
    ) -> fsscanner::Result<Vec<FileEntry>> {
        let proj_str = normalize_path(projdir).display().to_string();
        let build_path = format!("{}/build/", proj_str);
        let target_path = format!("{}/target/", proj_str);
        let mut selected_paths = Vec::<PathBuf>::with_capacity(256);
        for sel in selected {
            collect_files_all(sel, &mut selected_paths);
        }
        let mut selected_entries = FileEntry::vec_from_filtered_pathbufvec(None, selected_paths.to_vec())?;
        selected_entries.retain(|entry| {
            let path = entry.to_string();
            let Some(filename) = entry.path.file_name().and_then(|f| f.to_str()).map(String::from) else {
                return false;
            };
            let Some(suffix) = entry.path.extension().and_then(|s| s.to_str()).map(String::from) else {
                return false;
            };

            !path.contains(&build_path)
                && !path.contains(&target_path)
                && config.scanfullfilter.contains(&filename)
                && config.scanendfilter.contains(&suffix)
        });

        match workspacedir {
            Some(ws) => {
                let mut all_entries = Vec::<FileEntry>::with_capacity(128);

                for path in scan_with_suffix_and_filter(
                    ws,
                    &[".md", ".txt"],
                    filter,
                ) {
                    all_entries.push(FileEntry::from_path(&path)?);
                }

                for path in scan_with_suffix_and_filter(
                    ws,
                    &[],
                    filter,
                ) {
                    all_entries.push(FileEntry::from_path(&path)?);
                }
                all_entries.append(&mut selected_entries);
                Ok(all_entries)
            },
            None => Ok(selected_entries)
        }
    }

    pub fn max_workflow_attempts(&self) -> usize {
        self.config.max_try_count.max_workflow_fail
    }

    pub fn set_context(&self, context: impl Into<String>) {
        self.messages.borrow_mut().context = context.into();
    }

    pub fn set_task_description(&self, task_description: impl Into<String>) {
        self.messages.borrow_mut().task_description = task_description.into();
    }

    pub fn clear_context(&self) {
        self.messages.borrow_mut().context.clear();
    }

    pub fn run_text_step(&self, prompt: &str, context: &str) -> Result<String, LlmCallResult> {
        self.run_text_step_limited(prompt, context, 30000)
    }

    pub fn run_context_step_limited(
        &self,
        prompt: &str,
        context: &str,
        max_tokens: u32,
    ) -> Result<String, LlmCallResult> {
        self.run_text_step_with_context(prompt, context, max_tokens, true)
    }

    pub fn run_text_step_limited(
        &self,
        prompt: &str,
        context: &str,
        max_tokens: u32,
    ) -> Result<String, LlmCallResult> {
        self.run_text_step_with_context(prompt, context, max_tokens, false)
    }

    fn run_text_step_with_context(
        &self,
        prompt: &str,
        context: &str,
        max_tokens: u32,
        keep_source_context: bool,
    ) -> Result<String, LlmCallResult> {
        let endpoint = self.provider.endpoint.to_string();
        let air = AIRequest::new(
            &self.provider.model,
            endpoint,
            &self.provider.api_key,
            self.provider.insecure,
            max_tokens,
            0.6,
        );

        let json_messages = {
            let mut messages = self.messages.borrow().clone();
            messages.clear();
            messages.task_description = prompt.to_string();
            messages.subtask.clear();
            if !keep_source_context {
                messages.structureinfo.clear();
                messages.files.clear();
            }
            messages.context = context.to_string();
            messages.note.clear();
            messages.focus.clear();
            messages.to_json()
        };

        if self.dump {
            println!("### SEND");
            println!(
                "[run_text_step] {}",
                serde_json::to_string_pretty(&json_messages)
                    .unwrap_or_else(|_| "failed to encode json".to_string())
            );
            println!("### END");
        }

        let response = match air.request(&json_messages.to_string()) {
            AIRequestResult::Ok(value) => value,
            err => return Err(LlmCallResult::RequestError(err)),
        };

        if self.dump {
            println!("### RESPONSE");
            println!(
                "{}",
                serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "failed to decode json".to_string())
            );
        }

        let content = response
            .get("choices")
            .and_then(|value| value.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|content| !content.is_empty())
            .map(str::to_string);

        content.ok_or(LlmCallResult::RetryFailed)
    }

    pub fn run_until_done(&self, request: &str) -> LlmCallResult {
        let endpoint = self.provider.endpoint.to_string();
        let mut air = AIRequest::new(
            &self.provider.model,
            endpoint,
            &self.provider.api_key,
            self.provider.insecure,
            30000,
            0.6,
        );

        for _ in 0..self.config.max_try_count.max_workflow_fail {
            let result = self.analyze(&mut air, request);
            if result.is_done() || !result.is_valid() {
                return result;
            }
        }

        LlmCallResult::RetryFailed
    }

    pub fn run(&self, request: &str) -> LlmCallResult {
        const OK_CONFIRM_COUNT: usize = 2;

        let endpoint = self.provider.endpoint.to_string();
        let mut air = AIRequest::new(
            &self.provider.model,
            endpoint,
            &self.provider.api_key,
            self.provider.insecure,
            30000,
            0.6,
        );

        let max_attempts = self.config.max_try_count.max_workflow_fail;
        let mut ok_count = 0;
        let mut res = self.analyze(&mut air, request);

        for attempt in 1..max_attempts {
            if !res.is_valid() {
                ok_count = 0;
            } else {
                ok_count += 1;
                if ok_count >= OK_CONFIRM_COUNT {
                    break;
                }
            }

            res = self.analyze(&mut air, request);

            if attempt + 1 == max_attempts {
                break;
            }
        }

        res
    }

    // called by self.workflow => see workflows
    pub fn analyze(
        &self,
        air: &mut AIRequest,
        request: &str,
    ) -> LlmCallResult {
        {
           let mut messages = self.messages.borrow_mut();
           messages.cut_to_depth();
           messages.faults = Some(format!("=== PROBLEM ===\n{}", request));
        }
        for _ in 0..self.config.max_try_count.max_tool_call_fail {
            let response = {
                let messages = self.messages.borrow();
                let json_messages = messages.to_json();
                if self.dump {
                    println!("### SEND");
                    println!("[process_tool_chain] {}", serde_json::to_string_pretty(&json_messages).unwrap_or("failed to decode json".to_string()));
                    println!("### END");
                }

                match air.request(&json_messages.to_string()) {
                    AIRequestResult::Ok(val) => val,
                    err => return LlmCallResult::RequestError(err),
                }
            };

            if self.dump {
                println!("### RESPONSE");
                println!("{}", serde_json::to_string_pretty(&response).unwrap_or("failed to decode json".to_string()));
            }

            let choices = response.get("choices").and_then(|v| v.get(0));

            if let Some(choice) = choices {
/*
                let tool_calls = choice.get("tool_calls");
                if let Some(tc) = tool_calls {
                    let result = self.handle_native_tool_calls(tc);
                    if result.is_valid() {
                        return result;
                    }
                    continue;
                }
*/
                let content = choice
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                if self.dump {
                    println!("### CONTENT: {}", content);
                }

                if content.contains("action") {
                    let result = self.handle_text_action(content);
                    if result.is_valid() {
                        if result.to_base().is_done() {
                            return LlmCallResult::Done;
                        }
                        return LlmCallResult::ToolResult(result);
                    }
                    continue;
                }
            }

            return LlmCallResult::Done;
        }
        LlmCallResult::RetryFailed
    }
/*
    pub fn handle_native_tool_calls(&self, tool_calls: &Value) -> ToolBridgeResult {
        let mut rng = self.rng.borrow_mut();
        for call in tool_calls.as_array().unwrap_or(&vec![]) {
            let args = &call["function"]["arguments"];
            let real_id = call["id"].to_string();
            let result = execute_toolbridge_payload(&real_id, args.clone(), self.filter);
            let mut messages = self.messages.borrow_mut();

            if result.is_valid() {
                if result.tooltype.is_save() {
                    messages.clear();
                    messages.files = Self::create_files_info(
                        &self.config, self.projdir, self.workspacedir,
                        messages.task_id, self.filter, &mut rng
                    );
                    return result;
                }
                if result.tooltype.is_note() {
                    messages.note = result.toolresult["content"].to_string();
                }
                messages.append(&real_id, AIMessageType::Tool, result.tooltype, result.toolresult);
            } else {
                messages.append(&real_id, AIMessageType::Tool, result.tooltype, json!({
                    "role": "tool",
                    "tool_call_id": call["id"],
                    "content": format!("Error occurred: {}", result.valueerror.clone().expect("REASON"))
                }));
                return result;
            }
        }
        ToolBridgeResult::new_valid()
    }
*/
    pub fn handle_text_action(&self, content: &str) -> ToolOutput {
        let mut messages = self.messages.borrow_mut();
        let fake_id = messages.inc_messageid();
        // must not panic, just skip and retry
        let cleancode = strip_code_fences(&raw_fence_to_string(content));
        let json_result: Result<Value, serde_json::Error> = serde_json::from_str(&cleancode);

        let mut json = match json_result {
            Ok(v) => v,
            Err(e) => {
                // your error processing logic
                eprintln!("[handle_text_action] JSON parse error: {}\n>>>>CODE:\n{}\n<<<<", e, cleancode);
                messages.append(
                    fake_id, AIMessageType::Tool, AIToolType::Failed,
                    &format!("Error occurred: JSON parse error, this should contain a valid JSON block: {}\n>>>>CODE:\n{}\n<<<<", e, cleancode)
                );
                return ToolOutput::Failed(Failed::from_string(
                    format!("Error occurred: JSON parse error, this should contain a valid JSON block: {}\n>>>>CODE:\n{}\n<<<<", e, cleancode)
                ).execute());
            }
        };

        json["role"] = "assistant".into();
        if self.dump {
            println!("### CONTENT FOR TOOL: {}", cleancode);
        }
        let result = execute_tool(&json, &self.projdir, self.filter);

        messages.append(fake_id, AIMessageType::Model, result.to_base(), &cleancode);

        if result.is_valid() {
            if result.to_base().is_save() || result.to_base().is_done() {
                //println!("TOOL: {}", result.to_msg_string(fake_id));
                messages.clear();
                // TODO FIXME update only saved file
                if let Err(err) = messages.update() {
                    eprintln!("Failed to refresh file context: {err}");
                }
            }
            else {
                if result.to_base().is_failed() {
                    println!("TOOL: {}", result.to_msg_string(fake_id));
                }
                messages.append(fake_id, AIMessageType::Tool, result.to_base(), &result.to_msg_string(fake_id));
            }
        }
        else {
            messages.append(
                fake_id, AIMessageType::Tool, result.to_base(),
                &format!("Error occurred: {}", result.to_json(fake_id))
            );
        }

        result
    }
}
