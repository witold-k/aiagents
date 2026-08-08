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

#[expect(dead_code)]
pub struct WorkspaceWorkflow<'a> {
    bc: Buildcommand,
    projdir: &'a Path,
    workspacedir: &'a Path,
    targetdir: &'a Path,
}

impl<'a> WorkspaceWorkflow<'a> {
    pub fn new (
        bs: &Buildsystem,
        projdir: &'a Path,
        workspacedir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        WorkspaceWorkflow { bc: bs.build_cmd(workspacedir, targetdir), projdir, workspacedir, targetdir  }
    }
}

impl<'a> RunBuild for WorkspaceWorkflow<'a> {

    fn execute(
        &self,
        cb: &mut dyn FnMut(&str, &Path, &Path, &Buildresult) -> ToolOutput,
    ) -> Buildresult {
        println!("## BUILD");
        let br = run_cmd(self.workspacedir, &self.bc.build);
        if br.has_error() {
            _ = cb("build", self.workspacedir, self.targetdir, &br);
            return br;
        }
        println!("## LINT");
        let br = run_cmd(self.workspacedir, &self.bc.lint);
        if br. has_error() {
            _ = cb("lint", self.workspacedir, self.targetdir, &br);
            return br;
        }
        println!("## TEST");
        let br = run_cmd(self.workspacedir, &self.bc.test);
        if br. has_error() {
            _ = cb("test", self.workspacedir, self.targetdir, &br);
        }
        br
    }

}
