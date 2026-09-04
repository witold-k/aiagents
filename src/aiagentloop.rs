// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde_json::{Value};
use std::cell::RefCell;
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
    done::*,
    failed::*,
};
use crate::aimessage::{AIMessageId, AIMessageList, AIMessageType, AIMessageListData};
use crate::airequest::AIRequest;
use crate::workflows::{
    runbuild::RunBuild,
    buildresult::Buildresult
};
use crate::utils:: {
    ast::get_ast_string,
    jsonutils::get_json_field,
    scan_dir::scan_with_suffix_and_filter,
    stringutils::{strip_code_fences, raw_fence_to_string},
};
use crate::config::Config;
use crate::generated_tasks::Tasks;

#[expect(dead_code)]
pub struct AIAgentLoop<'a> {
    config: Config,
    projdir: PathBuf,
    workspacedir: Option<PathBuf>,
    filter: &'a Pathfilter,
    workflow: &'a dyn RunBuild,
    messages: RefCell<AIMessageList>,
    dump: bool,
}

impl<'a> AIAgentLoop<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Config,
        projdir: PathBuf,
        workspacedir: Option<PathBuf>,
        task_id: Tasks,
        task_description: String,
        subtask: Vec<String>,
        filter: &'a Pathfilter,
        selected: &'a [PathBuf],
        workflow: &'a dyn RunBuild,
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
            files: Self::create_files_info(
                &config, &projdir, workspacedir.as_deref(), task_id, filter, selected
            ),
            focus: "".into(),
            faults: None,
        };
        Self {
            config,
            projdir: normalize_path(&projdir),
            workspacedir,
            filter,
            workflow,
            messages: RefCell::new(AIMessageList::new(data)),
            dump,
        }
    }

    pub fn create_structure_info() -> String {
        get_ast_string("src")
    }

    pub fn create_files_info(
        config: &Config,
        projdir: &Path,
        workspacedir: Option<&Path>,
        _task_id: Tasks,
        filter: &'a Pathfilter,
        selected: &'a [PathBuf],
    ) -> Vec<FileEntry> {
        let proj_str = normalize_path(projdir).display().to_string();
        let build_path = format!("{}/build/", proj_str);
        let target_path = format!("{}/target/", proj_str);
        let mut selected_paths = Vec::<PathBuf>::with_capacity(256);
        for sel in selected {
            collect_files_all(sel, &mut selected_paths);
        }
        let mut selected_entries = FileEntry::vec_from_filtered_pathbufvec(None, selected_paths.to_vec());
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
                    all_entries.push(FileEntry::from_path(&path));
                }

                for path in scan_with_suffix_and_filter(
                    ws,
                    &[],
                    filter,
                ) {
                    all_entries.push(FileEntry::from_path(&path));
                }
                all_entries.append(&mut selected_entries);
                all_entries
            },
            None => selected_entries
        }
    }

    pub fn run(&self) {
        let provider = match self.config.get_selected_provider() {
            Some(provider) => provider,
            None           => return,
        };

        let endpoint = format!("{}/chat/completions", provider.endpoint);
        let mut air = AIRequest::new(
            &provider.model,
            endpoint,
            &provider.api_key,
            30000,
            0.6,
        );
        let mut okcount = 1;
        let mut cb = |name: &str, _p1: &Path, _p2: &Path, result: &Buildresult| {
            let res: ToolOutput = self.analyze(&mut air, name, result);

            if res.is_failed() {
                eprintln!("Tool Error occurred: {:?} =>\n{}", air, res);
            }

            res
        };

        let mut totalleft = self.config.max_try_count as isize;
        while okcount < 2 && totalleft > 0 {
            let br = self.workflow.execute(&mut cb);
            if br.has_error() {
                okcount = 0;
            }
            else {
                okcount += 1;
            }
            totalleft -= 1;
        }
    }

    // called by self.workflow => see workflows
    pub fn analyze(
        &self,
        air: &mut AIRequest,
        name: &str,
        result: &Buildresult
    ) -> ToolOutput {
        {
           let mut messages = self.messages.borrow_mut();
           messages.cut_to_depth();

           let build_result = result.limit_lines(100);

           if !build_result.has_error() {
               return ToolOutput::Done(Done::default().execute());
           }

           if !build_result.is_dummy() {
               messages.faults = Some(format!("=== {} OUTPUT ===\n{}", name, build_result));
           }
        }
        self.process_tool_chain(air)
    }

    pub fn process_tool_chain(&self, air: &mut AIRequest) -> ToolOutput {
        for _ in 0..10 {
            let response = {
                let messages = self.messages.borrow();
                if self.dump {
                    println!("### SEND");
                    println!("[process_tool_chain] {}", serde_json::to_string_pretty(&messages.to_json()).unwrap_or("failed to decode json".to_string()));
                    println!("### END");
                }

                match air.request(&messages.to_json().to_string()) {
                    Ok(v) => v,
                    Err(_) => {
                        let errtxt = format!(
                            "===================================================== >>>\n{}\n==========================================================\n{}\n<<< =====================================================\n",
                            messages.to_short_string(),
                            messages.to_json()
                        );
                        return ToolOutput::Failed(
                            Failed::from_string(errtxt).execute()
                        );
                    }
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
                if content.contains("action") {
                    let result = self.handle_text_action(content);
                    if result.is_valid() {
                        return result;
                    }
                    continue;
                }
            }

            return ToolOutput::Done(Done::default().execute());
        }
        ToolOutput::Failed(Failed::default().execute())
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
        // must not panic, just skip an d retry
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
        if json.get("content").is_none() {
            json["content"] = Value::Null;
        }
        let result = execute_tool(&json.clone(), &self.projdir, self.filter);

        let content = match get_json_field(&json, "content") {
            Ok(content) => content,
            Err(_)      => "".to_string(),
        };
        messages.append(fake_id, AIMessageType::Model, result.to_base(), &content);

        if result.is_valid() {
            if result.to_base().is_save() || result.to_base().is_done() {
                println!("TOOL: {}", result.to_string(fake_id));
                messages.clear();
            }
            else {
                if result.to_base().is_failed() {
                    println!("TOOL: {}", result.to_string(fake_id));
                }
                messages.append(fake_id, AIMessageType::Tool, result.to_base(), &fake_id.to_string());
            }
        }
        else {
            println!("[aiagentloop] ERROR: {}", result);
            messages.append(
                fake_id, AIMessageType::Tool, result.to_base(),
                &format!("Error occurred: {}", result.to_json(fake_id))
            );
        }

        result
    }
}
