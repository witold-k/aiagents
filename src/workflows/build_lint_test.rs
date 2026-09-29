// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing a project.

use std::cell::RefCell;
use std::path::Path;
use crate::config::Config;
use crate::generated_workflowsteps::WorkflowSteps;
use crate::utils::stringutils::{extract_standalone_choice, extract_standalone_keyword};
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
    previous_repair: RefCell<Option<String>>,
}

impl<'a> BLTWorkflow<'a> {
    pub fn new(
        config: &'a Config,
        llm_call: LlmCall<'a>,
        bs: Buildsystem,
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        BLTWorkflow {
            config,
            llm_call,
            bc: bs.build_cmd(projdir, targetdir),
            projdir,
            targetdir,
            previous_repair: RefCell::new(None),
        }
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

        let previous_repair = self.previous_repair.borrow().clone();
        let diagnostic_context = match &previous_repair {
            Some(previous) => format!(
                "=== PREVIOUS REPAIR ===\n{previous}\n\n=== CURRENT BUILD RESULT ===\n{diagnostic}"
            ),
            None => diagnostic.clone(),
        };

        println!("## [BLT] SELECT SOURCE CONTEXT");
        let selection = match self.llm_call.run_context_step_limited_with_temperature(
            WorkflowSteps::CodeFixSelectFiles.get_prompt(),
            &diagnostic,
            512,
            0.1,
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

        println!("## [BLT] DIAGNOSE FIX");
        let diagnosis = match self.llm_call.run_context_step_limited_with_temperature(
            WorkflowSteps::FixCodeDiagnose.get_prompt(),
            &diagnostic_context,
            1024,
            0.1,
        ) {
            Ok(diagnosis) => diagnosis,
            Err(result) => return WorkflowResult::LlmCallResult(result),
        };

        println!("## [BLT] DIAGNOSIS");
        println!("{diagnosis}");

        println!("## [BLT] DESIGN FIX");
        const MAX_ANALYSIS_ATTEMPTS: usize = 3;
        let mut design_request = format!(
            "=== DIAGNOSIS AND CONSTRAINTS ===\n{diagnosis}\n\n=== REPAIR CONTEXT ===\n{diagnostic_context}"
        );
        let mut fix_history = Vec::new();
        let mut fix_designs = Vec::new();

        for attempt in 1..=MAX_ANALYSIS_ATTEMPTS {
            let analysis = match self.llm_call.run_context_step_limited_with_temperature(
                WorkflowSteps::FixCodeAnalyze.get_prompt(),
                &design_request,
                2048,
                0.2,
            ) {
                Ok(analysis) => analysis,
                Err(result) => return WorkflowResult::LlmCallResult(result),
            };


            println!("## [BLT] FIX DESIGN {attempt}");
            println!("{analysis}");

            println!("## [BLT] CRITIQUE FIX {attempt}");
            let critique_request = format!(
                "=== DIAGNOSIS AND CONSTRAINTS ===\n{diagnosis}\n\n=== CANDIDATE FIX DESIGN ===\n{analysis}\n\n=== REPAIR CONTEXT ===\n{diagnostic_context}"
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

            let decision = match extract_standalone_keyword(&critique, &["ACCEPT", "REJECT"]).as_deref() {
                Some("ACCEPT") => "ACCEPT",
                Some("REJECT") => "REJECT",
                _ => {
                    println!("## [BLT] CRITIQUE INVALID RESPONSE");
                    return WorkflowResult::LlmCallResult(LlmCallResult::RetryFailed);
                },
            };

            fix_designs.push(analysis.clone());
            fix_history.push(format!(
                "=== FIX DESIGN {attempt} [{decision}] ===\n{analysis}\n\n=== CRITIQUE {attempt} ===\n{critique}"
            ));

            if decision == "ACCEPT" {
                break;
            }

            design_request = format!(
                "=== DIAGNOSIS AND CONSTRAINTS ===\n{diagnosis}\n\n=== REPAIR CONTEXT ===\n{diagnostic_context}\n\n=== REJECTED DESIGN CRITIQUE ===\n{critique}\n\nProduce a different repair mechanism that satisfies the same diagnosis and constraints."
            );
        }

        println!("## [BLT] SYNTHESIZE FIX");
        let synthesis_request = format!(
            "=== DIAGNOSIS AND CONSTRAINTS ===\n{diagnosis}\n\n=== REPAIR CONTEXT ===\n{diagnostic_context}\n\n=== FIX DESIGN HISTORY ===\n{}",
            fix_history.join("\n\n")
        );
        let synthesis = match self.llm_call.run_context_choice_step(
            WorkflowSteps::FixCodeSynthesize.get_prompt(),
            &synthesis_request,
            fix_designs.len(),
        ) {
            Ok(selection) => selection,
            Err(result) => return WorkflowResult::LlmCallResult(result),
        };

        let Some(choice) = extract_standalone_choice(&synthesis, fix_designs.len()) else {
            println!("## [BLT] SYNTHESIS INVALID RESPONSE");
            return WorkflowResult::LlmCallResult(LlmCallResult::RetryFailed);
        };
        let final_plan = &fix_designs[choice - 1];

        println!("## [BLT] FINAL FIX PLAN: DESIGN {choice}");
        println!("{final_plan}");

        println!("## [BLT] APPLY FIX");
        self.llm_call.set_context(format!(
            "=== FINAL FIX PLAN ===\n{final_plan}\n\n\
             === APPLY INSTRUCTIONS ===\n\
             Apply this fix plan completely before returning done.\n\
             A successful save clears previously loaded transient source files to keep context bounded.\n\
             If another edit requires source that is no longer loaded, use load_file to reload only the file needed for that edit.\n\
             Do not start unrelated repairs or reconsider rejected alternatives."
        ));

        let diagnostic = buildresult.limit_lines(100).to_string();
        let result = self.llm_call.run_fix_step(&diagnostic);
        println!("## [BLT] APPLY RESULT: {result}");

        if result.is_valid() {
            let completed_edits = self.llm_call.completed_edits();
            if !completed_edits.is_empty() {
                *self.previous_repair.borrow_mut() = Some(format!(
                    "Selected plan:\n{final_plan}\n\nApplied edits:\n{}",
                    completed_edits.join("\n\n")
                ));
            }
        }

        if result.is_valid() {
            WorkflowResult::BuildResult(buildresult)
        } else {
            WorkflowResult::LlmCallResult(result)
        }
    }
}

