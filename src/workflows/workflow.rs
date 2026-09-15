// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::runtimetools::{
    buildresult::Buildresult,
    llmcall::{LlmCall, LlmCallResult},
};

// FIXME general: Workflow should be renamed to WorkflowResult

pub enum WorkflowResult {
    Ok,
    LlmCallResult(LlmCallResult),
}

impl WorkflowResult {
    pub fn success(&self) -> bool {
        match self {
            Self::Ok => true,
            Self::LlmCallResult(result) => result.is_valid(),
        }
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
            let res: LlmCallResult = llm_call.run(&buildresult.to_string());
            WorkflowResult::LlmCallResult(res)
        }
        else {
            WorkflowResult::Ok
        }
    }
}

