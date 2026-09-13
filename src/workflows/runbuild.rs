// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde::{Serialize, Deserialize};
use crate::runtimetools::{
    buildresult::Buildresult,
    llmcall::LlmCall,
};
use crate::agenttools::{
    all_tools::ToolOutput,
};

// FIXME general: RunBuild should be renamed to WorkflowResult

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum RunBuildResult {
    Ok,
    Failed
}

impl RunBuildResult {
    pub fn success(self) -> bool {
        self == RunBuildResult::Ok
    }
}

pub trait RunBuild {
    fn execute(
        &self
    ) -> RunBuildResult;

    fn execute_build_llm(
        &self,
        buildresult: &Buildresult,
        llm_call: &LlmCall
    ) -> RunBuildResult {
        if buildresult.has_error() {
            let _res: ToolOutput = llm_call.run(&buildresult.to_string());
            RunBuildResult::Failed
        }
        else {
            RunBuildResult::Ok
        }
    }

}

