// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! this is only a helper class for the main function
//! to easy select the workflow that is associated to a task

use std::path::Path;
use crate::workflows::{
    build::BuildWorkflow,
    build_lint_test::BLTWorkflow,
    buildsystem::Buildsystem,
    doc::DocWorkflow,
    endless::EndlessWorkflow,
    setupbuild::SetupBuildWorkflow,
    workspace::WorkspaceWorkflow,
    runbuild::RunBuild,
};
use crate::generated_tasks::Tasks;

#[expect(dead_code)]
pub struct WorkflowSelector<'a> {
    build: BuildWorkflow<'a>,
    bt: BLTWorkflow<'a>,
    gt: EndlessWorkflow<'a>,
    dt: DocWorkflow<'a>,
    tt: EndlessWorkflow<'a>,
    sb: SetupBuildWorkflow<'a>,
    ws: WorkspaceWorkflow<'a>,
}

impl<'a> WorkflowSelector<'a> {

    pub fn new(
        bs: &'a Buildsystem,
        src_path: &'a Path,
        workspace_path: &'a Path,
        target_path: &'a Path,
    ) -> Self {
        Self {
            build: BuildWorkflow::new(bs, src_path, target_path),
            bt: BLTWorkflow::from_buildsystem(bs, src_path, target_path),
            gt: EndlessWorkflow::new(src_path, target_path),
            dt: DocWorkflow::from_buildsystem(bs, src_path, target_path),
            tt: EndlessWorkflow::new(src_path, target_path),
            sb: SetupBuildWorkflow::new(bs, src_path, target_path),
            ws: WorkspaceWorkflow::new(bs, src_path, workspace_path, target_path),
        }
    }

    pub fn select(&self, aitask: Tasks) -> &dyn RunBuild {
        match aitask {
            Tasks::Analyze        => &self.dt,
            Tasks::Build          => &self.build,
            Tasks::FixCode        => &self.bt,
            Tasks::GenCode        => &self.gt,
            Tasks::ReviewCode     => &self.dt,
            Tasks::ReviewDoc      => &self.dt,
            Tasks::SetupBuild     => &self.sb,
            Tasks::TranspileCode  => &self.tt,
            Tasks::WriteBlockDoc  => &self.dt,
            Tasks::WriteItemDoc   => &self.dt,
            Tasks::WriteModuleDoc => &self.dt,
            Tasks::WriteTestCode  => &self.bt,
        }
    }
}

