// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing
// it is a "single shot" command without any postprocessing, unlike build_lint_test

use std::path::Path;
use crate::config::Config;
use crate::runtimetools::{
    llmcall::LlmCall,
    buildresult::Buildresult,
    buildsystem::{Buildsystem, Buildcommand},
    generic_work_step::run_cmd,
};
use crate::workflows::{
    runbuild::RunBuild,
    runbuild::RunBuildResult,
};

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
        BuildWorkflow { config, llm_call, bc: bs.build_cmd(projdir, targetdir), projdir }
    }

    /// returns build error, if any
    pub fn build(&self) -> Buildresult {
        println!("## BUILD");
        let br = run_cmd(self.projdir, &self.bc.build);
        if br.has_error() {
            return br;
        }

        br
    }
}

impl<'a> RunBuild for BuildWorkflow<'a> {
    fn execute(
        &self,
    ) -> RunBuildResult {
        self.execute_build_llm(&self.build(), &self.llm_call)
    }
}

