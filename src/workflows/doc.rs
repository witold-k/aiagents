// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing a project.

use std::path::Path;
use crate::agenttools::all_tools::ToolOutput;
use crate::repostate::{
    RepoState,
    gitstate::GitState,
};
use crate::workflows::{
    runbuild::RunBuild,
    buildresult::Buildresult,
    buildsystem::{Buildsystem, Buildcommand},
    generic_work_step::run_cmd,
};

pub struct DocWorkflow<'a> {
    bc: Buildcommand,
    projdir: &'a Path,
    targetdir: &'a Path,
    state: GitState,
}

impl<'a> DocWorkflow<'a> {
    pub fn from_buildsystem(
        bs: &Buildsystem,
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        DocWorkflow {
            bc: bs.build_cmd(projdir, targetdir),
            projdir, targetdir,
            state: GitState::from_path(projdir)
        }
    }
}

impl<'a> RunBuild for DocWorkflow<'a> {

    fn execute(
        &self,
        cb: &mut dyn FnMut(&str, &Path, &Path, &Buildresult) -> ToolOutput,
    ) -> Buildresult {
        // first check: is it building at all
        let br = run_cmd(self.projdir, &self.bc.build);
        if br.has_error() {
            self.state.restore();
        }
        let br = run_cmd(self.projdir, &self.bc.lint);
        if br.has_error() {
            self.state.restore();
        }

        // no errors occured => commit current state
        self.state.commit();

        let br = Buildresult::new_need_build();
        let res = cb("doc", self.projdir, self.targetdir, &br);
        if res.is_done() { Buildresult::new_no_build() } else { br }
    }

}
