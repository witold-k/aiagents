// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing a project.

use std::path::Path;
use crate::agenttools::all_tools::ToolOutput;
use crate::workflows::{
    runbuild::RunBuild,
    buildresult::Buildresult,
    buildsystem::{Buildsystem, Buildcommand},
    generic_work_step::run_cmd,
};

pub struct BLTWorkflow<'a> {
    bc: Buildcommand,
    projdir: &'a Path,
    targetdir: &'a Path,
}

impl<'a> BLTWorkflow<'a> {
    pub fn from_buildsystem(
        bs: &Buildsystem,
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        BLTWorkflow { bc: bs.build_cmd(projdir, targetdir), projdir, targetdir  }
    }
}

impl<'a> RunBuild for BLTWorkflow<'a> {

    fn execute(
        &self,
        cb: &mut dyn FnMut(&str, &Path, &Path, &Buildresult) -> ToolOutput,
    ) -> Buildresult {
        println!("## BUILD");
        let br = run_cmd(self.projdir, &self.bc.build);
        if br.has_error() {
            _ = cb("build", self.projdir, self.targetdir, &br);
            return br;
        }
        println!("## LINT");
        let br = run_cmd(self.projdir, &self.bc.lint);
        if br.has_error() {
            _ = cb("lint", self.projdir, self.targetdir, &br);
            return br;
        }
        println!("## TEST");
        let br = run_cmd(self.projdir, &self.bc.test);
        if br.has_error() {
            _ = cb("test", self.projdir, self.targetdir, &br);
        }
        br
    }

}
