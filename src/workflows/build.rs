// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing
// it is a "single shot" command without any postprocessing, unlike build_lint_test

use crate::config::Config;
use crate::runtimetools::{
    buildresult::Buildresult,
    buildsystem::{Buildcommand, Buildsystem},
    generic_work_step::run_cmd,
    llmcall::LlmCall,
};
use crate::workflows::{workflow::Workflow, workflow::WorkflowResult};
use std::path::Path;

#[expect(dead_code)]
pub struct BuildWorkflow<'a> {
    config: &'a Config,
    llm_call: LlmCall<'a>,
    bc: Buildcommand,
    projdir: &'a Path,
}

impl<'a> BuildWorkflow<'a> {
    pub fn new(
        config: &'a Config,
        llm_call: LlmCall<'a>,
        bs: Buildsystem,
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        BuildWorkflow {
            config,
            llm_call,
            bc: bs.build_cmd(projdir, targetdir),
            projdir,
        }
    }

    /// returns build error, if any
    pub fn build(&self) -> Buildresult {
        println!("## [BUILD] BUILD");
        let br = run_cmd(self.projdir, &self.bc.build);
        if br.has_error() {
            return br;
        }

        br
    }
}

impl<'a> Workflow for BuildWorkflow<'a> {
    fn execute(&self) -> WorkflowResult {
        self.execute_build_llm(&self.build(), &self.llm_call)
    }
}
