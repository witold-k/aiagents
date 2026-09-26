// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing a project.

use std::path::Path;
use crate::config::Config;
use crate::generated_workflowsteps::WorkflowSteps;
use crate::runtimetools::{
    llmcall::LlmCall,
    buildresult::Buildresult,
    buildsystem::{Buildsystem, Buildcommand},
    generic_work_step::run_cmd,
};
use crate::workflows::{
    workflow::Workflow,
    workflow::WorkflowResult,
};

#[expect(dead_code)]
pub struct BLTWorkflow<'a> {
    config: &'a Config,
    llm_call: LlmCall<'a>,
    bc: Buildcommand,
    projdir: &'a Path,
    targetdir: &'a Path,
}

impl<'a> BLTWorkflow<'a> {
    pub fn new(
        config: &'a Config,
        llm_call: LlmCall<'a>,
        bs: Buildsystem,
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        BLTWorkflow { config, llm_call, bc: bs.build_cmd(projdir, targetdir), projdir, targetdir }
    }

    /// returns build error, if any
    pub fn build(&self) -> Buildresult {
        println!("## [BLT] BUILD");
        let br = run_cmd(self.projdir, &self.bc.build);
        if br.has_error() {
            return br;
        }
        println!("## [BLT] LINT");
        let br = run_cmd(self.projdir, &self.bc.lint);
        if br.has_error() {
            return br;
        }
        println!("## [BLT] TEST");
        let br = run_cmd(self.projdir, &self.bc.test);
        if br.has_error() {
            return br;
        }

        br
    }
}

impl<'a> Workflow for BLTWorkflow<'a> {
    fn execute(
        &self,
    ) -> WorkflowResult {
        let buildresult = self.build();
        if !buildresult.has_error() {
            return WorkflowResult::Ok;
        }

        let diagnostic = buildresult.to_string();
        println!("## [BLT] ANALYZE FIX");
        let analysis = match self.llm_call.run_context_step_limited(
            WorkflowSteps::FixCodeAnalyze.get_prompt(),
            &diagnostic,
            2048,
        ) {
            Ok(analysis) => analysis,
            Err(result) => return WorkflowResult::LlmCallResult(result),
        };

        println!("## [BLT] FIX PLAN");
        println!("{analysis}");
        println!("## [BLT] APPLY FIX");
        self.llm_call.set_context(format!(
            "=== FIX ANALYSIS ===\n{analysis}"
        ));

        let result = self.execute_build_llm(&buildresult, &self.llm_call);
        println!("## [BLT] APPLY RESULT: {result}");
        result
    }
}

