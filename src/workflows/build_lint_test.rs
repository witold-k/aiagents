// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing a project.

use std::path::Path;
use crate::config::Config;
use crate::generated_workflowsteps::WorkflowSteps;
use crate::utils::stringutils::extract_standalone_keyword;
use crate::runtimetools::{
    llmcall::{LlmCall, LlmCallResult},
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

        let diagnostic = buildresult.limit_lines(100).to_string();
        self.llm_call.update_structure_info(&diagnostic);

        println!("## [BLT] SELECT SOURCE CONTEXT");
        let selection = match self.llm_call.run_context_step_limited(
            WorkflowSteps::CodeFixSelectFiles.get_prompt(),
            &diagnostic,
            512,
        ) {
            Ok(selection) => selection,
            Err(result) => return WorkflowResult::LlmCallResult(result),
        };

        match self.llm_call.load_selected_source_files(&selection, 2) {
            Ok(count) => println!("## [BLT] SOURCE CONTEXT: {count} files loaded"),
            Err(err) => {
                eprintln!("## [BLT] SOURCE CONTEXT ERROR: {err}");
            },
        }

        println!("## [BLT] ANALYZE FIX");
        const MAX_ANALYSIS_ATTEMPTS: usize = 3;
        let mut analysis_request = diagnostic.clone();
        let mut accepted_analysis = None;

        for attempt in 1..=MAX_ANALYSIS_ATTEMPTS {
            let analysis = match self.llm_call.run_context_step_limited_with_temperature(
                WorkflowSteps::FixCodeAnalyze.get_prompt(),
                &analysis_request,
                2048,
                0.2,
            ) {
                Ok(analysis) => analysis,
                Err(result) => return WorkflowResult::LlmCallResult(result),
            };

            println!("## [BLT] FIX PLAN {attempt}");
            println!("{analysis}");

            println!("## [BLT] CRITIQUE FIX {attempt}");
            let critique_request = format!(
                "=== CANDIDATE FIX ANALYSIS ===\n{analysis}\n\n=== ORIGINAL DIAGNOSTIC ===\n{diagnostic}"
            );
            let critique = match self.llm_call.run_context_step_limited_with_temperature(
                WorkflowSteps::FixCodeCritique.get_prompt(),
                &critique_request,
                1024,
                0.1,
            ) {
                Ok(critique) => critique,
                Err(result) => return WorkflowResult::LlmCallResult(result),
            };

            println!("## [BLT] CRITIQUE RESULT {attempt}");
            println!("{critique}");

            match extract_standalone_keyword(&critique, &["ACCEPT", "REJECT"]).as_deref() {
                Some("ACCEPT") => {
                    accepted_analysis = Some(analysis);
                    break;
                },
                Some("REJECT") => {},
                _ => {
                    println!("## [BLT] CRITIQUE INVALID RESPONSE");
                    return WorkflowResult::LlmCallResult(LlmCallResult::RetryFailed);
                },
            }

            analysis_request = format!(
                "{diagnostic}\n\n=== PREVIOUS CANDIDATE FIX ANALYSIS ===\n{analysis}\n\n=== CRITIQUE ===\n{critique}\n\nProduce a new fix analysis that addresses the critique. Do not repeat the rejected repair."
            );
        }

        let Some(analysis) = accepted_analysis else {
            println!("## [BLT] FIX PLAN REJECTED AFTER {MAX_ANALYSIS_ATTEMPTS} ATTEMPTS");
            return WorkflowResult::LlmCallResult(LlmCallResult::RetryFailed);
        };

        println!("## [BLT] APPLY FIX");
        self.llm_call.set_context(format!(
            "=== FIX ANALYSIS ===\n{analysis}"
        ));

        let diagnostic = buildresult.limit_lines(100).to_string();
        let result = self.llm_call.run_fix_step(&diagnostic);
        println!("## [BLT] APPLY RESULT: {result}");

        if result.is_valid() {
            WorkflowResult::BuildResult(buildresult)
        } else {
            WorkflowResult::LlmCallResult(result)
        }
    }
}

