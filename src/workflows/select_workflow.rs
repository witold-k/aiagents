// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! this is only a helper class for the main function
//! to easy select the workflow that is associated to a task

use std::path::Path;
use crate::config::Config;
use crate::runtimetools::{
    llmcall::LlmCall,
    buildsystem::Buildsystem,
};
use crate::workflows::{
    build::BuildWorkflow,
    build_lint_test::BLTWorkflow,
    doc::DocWorkflow,
    endless::EndlessWorkflow,
    release_doc::ReleaseDocWorkflow,
    setupbuild::SetupBuildWorkflow,
    workspace::WorkspaceWorkflow,
    workflow::Workflow,
};
use crate::generated_tasks::Tasks;

#[expect(dead_code)]
pub struct WorkflowSelector<'a> {
    build: BuildWorkflow<'a>,
    bt: BLTWorkflow<'a>,
    gt: EndlessWorkflow<'a>,
    dt: DocWorkflow<'a>,
    rd: ReleaseDocWorkflow<'a>,
    tt: EndlessWorkflow<'a>,
    sb: SetupBuildWorkflow<'a>,
    ws: WorkspaceWorkflow<'a>,
}

// FIXME should be named WorkflowBuilder and behave so
impl<'a> WorkflowSelector<'a> {

    pub fn new(
        config: &'a Config,
        llm_call: &'a LlmCall<'a>,
        bs: Buildsystem,
        src_path: &'a Path,
        workspace_path: &'a Path,
        target_path: &'a Path,
        task_args: &[String],
    ) -> Self {
        Self {
            build: BuildWorkflow::new(config, llm_call.clone(), bs, src_path, target_path),
            bt: BLTWorkflow::new(config, llm_call.clone(), bs, src_path, target_path),
            gt: EndlessWorkflow::new(config, llm_call.clone(), src_path, target_path),
            dt: DocWorkflow::new(config, llm_call.clone(), bs, src_path, target_path),
            rd: ReleaseDocWorkflow::new(llm_call.clone(), src_path, task_args),
            tt: EndlessWorkflow::new(config, llm_call.clone(), src_path, target_path),
            sb: SetupBuildWorkflow::new(config, llm_call.clone(), bs, src_path, target_path),
            ws: WorkspaceWorkflow::new(config, llm_call.clone(), bs, src_path, workspace_path, target_path),
        }
    }

    pub fn select(&self, aitask: Tasks) -> &dyn Workflow {
        match aitask {
            Tasks::Analyze        => &self.dt,
            Tasks::Build          => &self.build,
            Tasks::FixCode        => &self.bt,
            Tasks::GenCode        => &self.gt,
            Tasks::ReviewCode     => &self.dt,
            Tasks::ReviewDoc      => &self.dt,
            Tasks::ReleaseDoc     => &self.rd,
            Tasks::SetupBuild     => &self.sb,
            Tasks::TranspileCode  => &self.tt,
            Tasks::WriteBlockDoc  => &self.dt,
            Tasks::WriteItemDoc   => &self.dt,
            Tasks::WriteModuleDoc => &self.dt,
            Tasks::WriteTestCode  => &self.bt,
        }
    }
}

