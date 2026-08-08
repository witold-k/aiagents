// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

// This file contains functions for building, linting, and testing
// it is a "single shot" command without any postprocessing, unlike build_lint_test

use std::path::Path;
use crate::agenttools::all_tools::ToolOutput;
use crate::workflows::{
    runbuild::RunBuild,
    buildresult::Buildresult,
    buildsystem::{Buildsystem, Buildcommand},
    generic_work_step::run_cmd,
};

pub struct BuildWorkflow<'a> {
    bc: Buildcommand,
    projdir: &'a Path,
}

impl<'a> BuildWorkflow<'a> {
    pub fn new(
        bs: &Buildsystem,
        projdir: &'a Path,
        targetdir: &'a Path,
    ) -> Self {
        BuildWorkflow { bc: bs.build_cmd(projdir, targetdir), projdir  }
    }
}

impl<'a> RunBuild for BuildWorkflow<'a> {

    fn execute(
        &self,
        _cb: &mut dyn FnMut(&str, &Path, &Path, &Buildresult) -> ToolOutput,
    ) -> Buildresult {
        println!("## BUILD");
        let br = run_cmd(self.projdir, &self.bc.build);
        if br.has_error() {
            return Buildresult::new_no_build();
        }
        println!("## LINT");
        let br = run_cmd(self.projdir, &self.bc.lint);
        if br. has_error() {
            return Buildresult::new_no_build();
        }
        println!("## TEST");
        run_cmd(self.projdir, &self.bc.test);
        Buildresult::new_no_build()
    }

}
