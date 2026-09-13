// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing a project.

use std::path::Path;
use crate::config::Config;
use crate::runtimetools::{
    llmcall::LlmCall,
    buildsystem::Buildsystem,
};
use crate::workflows::{
    runbuild::RunBuild,
    runbuild::RunBuildResult,
};

#[expect(dead_code)]
pub struct SetupBuildWorkflow<'a> {
    config: &'a Config,
    llm_call: LlmCall<'a>,
    bs: Buildsystem,
    projdir: &'a Path,
    targetdir: &'a Path,
}

impl<'a> SetupBuildWorkflow<'a> {
    pub fn new(
        config: &'a Config,
        llm_call: LlmCall<'a>,
        bs: Buildsystem,
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        SetupBuildWorkflow {
            config,
            llm_call,
            bs,
            projdir,
            targetdir,
        }
    }
}

impl<'a> RunBuild for SetupBuildWorkflow<'a> {
    fn execute(
        &self,
    ) -> RunBuildResult {
        self.bs.setupbuild(self.projdir, self.targetdir);
        RunBuildResult::Ok
    }
}
