// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use crate::runtimetools::{
    buildresult::Buildresult,
    llmcall::{LlmCall, LlmCallResult},
};

pub enum WorkflowResult {
    Ok,
    LlmCallResult(LlmCallResult),
    BuildResult(Buildresult),
    Error(String),
}

impl WorkflowResult {
    pub fn success(&self) -> bool {
        match self {
            Self::Ok => true,
            Self::LlmCallResult(result) => result.is_valid(),
            Self::BuildResult(result) => !result.has_error(),
            Self::Error(_) => false,
        }
    }

    pub fn request_failure(&self) -> bool {
        match self {
            Self::LlmCallResult(result) => result.is_request_error(),
            _ => false
        }
    }

}

impl fmt::Display for WorkflowResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "Ok"),
            Self::LlmCallResult(result) => write!(f, "{result}"),
            Self::BuildResult(result) => write!(f, "{result}"),
            Self::Error(error) => write!(f, "{error}"),
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
            if res.is_valid() {
                WorkflowResult::BuildResult(buildresult.clone())
            }
            else {
                WorkflowResult::LlmCallResult(res)
            }
        }
        else {
            WorkflowResult::Ok
        }
    }
}
