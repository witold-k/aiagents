// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::{PathBuf};
use fsscanner::{
    pathfilter::Pathfilter,
    pathutils::normalize_path,
};
use crate::workflows::workflow::Workflow;
use crate::config::Config;

#[expect(dead_code)]
pub struct AIAgentLoop<'a> {
    config: Config,
    projdir: PathBuf,
    workspacedir: Option<PathBuf>,
    filter: &'a Pathfilter,
    workflow: &'a dyn Workflow,
    dump: bool,
}

impl<'a> AIAgentLoop<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Config,
        projdir: PathBuf,
        workspacedir: Option<PathBuf>,
        filter: &'a Pathfilter,
        workflow: &'a dyn Workflow,
        dump: bool,
    ) -> Self {
        Self {
            config,
            projdir: normalize_path(&projdir),
            workspacedir,
            filter,
            workflow,
            dump,
        }
    }

    pub fn run(&self) {
        let mut okcount = 1;
        let mut totalleft = self.config.max_try_count.max_workflow_fail as isize;
        while okcount < 2 && totalleft > 0 {
            let br = self.workflow.execute();
            if !br.success() {
                okcount = 0;
            }
            else {
                okcount += 1;
            }
            totalleft -= 1;
        }
    }
}
