// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing a project.

use std::path::Path;
use crate::config::Config;
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
pub struct WorkspaceWorkflow<'a> {
    config: &'a Config,
    llm_call: LlmCall<'a>,
    bc: Buildcommand,
    projdir: &'a Path,
    workspacedir: &'a Path,
    targetdir: &'a Path,
}

impl<'a> WorkspaceWorkflow<'a> {
    pub fn new (
        config: &'a Config,
        llm_call: LlmCall<'a>,
        bs: Buildsystem,
        projdir: &'a Path,
        workspacedir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        WorkspaceWorkflow { config, llm_call, bc: bs.build_cmd(workspacedir, targetdir), projdir, workspacedir, targetdir  }
    }

    /// returns build error, if any
    pub fn build(&self) -> Buildresult {
        println!("## [WS] BUILD");
        let br = run_cmd(self.projdir, &self.bc.build);
        if br.has_error() {
            return br;
        }
        println!("## [WS] LINT");
        let br = run_cmd(self.projdir, &self.bc.lint);
        if br.has_error() {
            return br;
        }
        println!("## [WS] TEST");
        let br = run_cmd(self.projdir, &self.bc.test);
        if br.has_error() {
            return br;
        }

        br
    }
}

impl<'a> Workflow for WorkspaceWorkflow<'a> {
    fn execute(
        &self,
    ) -> WorkflowResult {
        self.execute_build_llm(&self.build(), &self.llm_call)
    }
}
