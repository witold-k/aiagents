// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::Path;
use crate::config::Config;
use crate::runtimetools::{
    llmcall::LlmCall,
};
use crate::agenttools::{
    all_tools::ToolOutput,
};
use crate::workflows::{
    runbuild::RunBuild,
    runbuild::RunBuildResult,
};

#[expect(dead_code)]
pub struct EndlessWorkflow<'a> {
    config: &'a Config,
    llm_call: LlmCall<'a>,
    projdir: &'a Path,
    targetdir: &'a Path,
}

impl<'a> EndlessWorkflow<'a> {
    pub fn new(
        config: &'a Config,
        llm_call: LlmCall<'a>,
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        EndlessWorkflow { config, llm_call, projdir, targetdir  }
    }
}

impl<'a> RunBuild for EndlessWorkflow<'a> {
    fn execute(
        &self,
    ) -> RunBuildResult {
        let res: ToolOutput = self.llm_call.run("");
        if res.is_done() {
            RunBuildResult::Ok
        } else {
            RunBuildResult::Failed
        }
    }
}
