// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::{PathBuf};
use fsscanner::{
    pathfilter::Pathfilter,
    pathutils::normalize_path,
};
use crate::workflows::workflow::{Workflow, WorkflowResult};
use crate::config::Config;

const REQUIRED_CONSECUTIVE_SUCCESSES: usize = 2;
const INITIAL_SUCCESS_COUNT: usize = 1;

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
        let mut okcount = INITIAL_SUCCESS_COUNT;
        let mut totalleft = self.config.max_try_count.max_workflow_fail as isize;
        while okcount < REQUIRED_CONSECUTIVE_SUCCESSES && totalleft > 0 {
            let wr: WorkflowResult = self.workflow.execute();
            if !wr.success() {
                okcount = 0;
                eprintln!("FAIL: {}", wr);
                if wr.request_failure() {
                    println!("{}", wr);
                }
            }
            else {
                //println!("{}", wr);
                okcount += 1;
            }
            totalleft -= 1;
        }
    }
}
