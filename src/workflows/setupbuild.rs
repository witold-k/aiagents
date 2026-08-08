// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing a project.

use std::path::Path;
use crate::agenttools::all_tools::ToolOutput;
use crate::workflows::{
    runbuild::RunBuild,
    buildsystem::Buildsystem,
    buildresult::Buildresult,
};

pub struct SetupBuildWorkflow<'a> {
    bs: &'a Buildsystem,
    projdir: &'a Path,
    targetdir: &'a Path,
}

impl<'a> SetupBuildWorkflow<'a> {
    pub fn new(
        bs: &'a Buildsystem,
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        SetupBuildWorkflow {
            bs,
            projdir,
            targetdir,
        }
    }
}

impl<'a> RunBuild for SetupBuildWorkflow<'a> {

    fn execute(
        &self,
        _cb: &mut dyn FnMut(&str, &Path, &Path, &Buildresult) -> ToolOutput,
    ) -> Buildresult {
        self.bs.setupbuild(self.projdir, self.targetdir);
        Buildresult::new_no_build()
    }

}
