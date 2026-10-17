// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing a project.

use crate::config::Config;
use crate::runtimetools::{buildsystem::Buildsystem, llmcall::LlmCall};
use crate::workflows::{workflow::Workflow, workflow::WorkflowResult};
use std::path::Path;

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

impl<'a> Workflow for SetupBuildWorkflow<'a> {
    fn execute(&self) -> WorkflowResult {
        self.bs.setupbuild(self.projdir, self.targetdir);
        WorkflowResult::Ok
    }
}
