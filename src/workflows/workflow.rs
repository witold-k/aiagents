// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::runtimetools::{
    buildresult::Buildresult,
    llmcall::LlmCall,
};
use crate::agenttools::{
    all_tools::ToolOutput,
};

// FIXME general: Workflow should be renamed to WorkflowResult

pub enum WorkflowResult {
    Ok,
    LlmCallFailed(ToolOutput),
}

impl WorkflowResult {
    pub fn success(&self) -> bool {
        matches!(self, WorkflowResult::Ok)
    }
}

pub trait Workflow {
    fn execute(
        &self
    ) -> WorkflowResult;

    fn execute_build_llm(
        &self,
        buildresult: &Buildresult,
        llm_call: &LlmCall
    ) -> WorkflowResult {
        if buildresult.has_error() {
            let res: ToolOutput = llm_call.run(&buildresult.to_string());
            WorkflowResult::LlmCallFailed(res)
        }
        else {
            WorkflowResult::Ok
        }
    }

}

