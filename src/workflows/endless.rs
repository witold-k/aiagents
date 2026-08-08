// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::Path;
use crate::agenttools::all_tools::ToolOutput;
use crate::workflows::{
    buildresult::Buildresult,
    runbuild::RunBuild
};

pub struct EndlessWorkflow<'a> {
    projdir: &'a Path,
    targetdir: &'a Path,
}

impl<'a> EndlessWorkflow<'a> {
    pub fn new(
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        EndlessWorkflow { projdir, targetdir  }
    }
}

impl<'a> RunBuild for EndlessWorkflow<'a> {

    fn execute(
        &self,
        cb: &mut dyn FnMut(&str, &Path, &Path, &Buildresult) -> ToolOutput,
    ) -> Buildresult {
        let br = Buildresult::new_need_build();
        _ = cb("generic", self.projdir, self.targetdir, &br);
        br
    }

}
