// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::Path;

use crate::runtimetools::{
    llmcall::{LlmCall, LlmCallResult},
    releasedoc::ReleaseDocContext,
};
use crate::vc::git::Git;
use crate::workflows::workflow::{Workflow, WorkflowResult};

pub struct ReleaseDocWorkflow<'a> {
    llm_call: LlmCall<'a>,
    projdir: &'a Path,
    task_args: Vec<String>,
}

impl<'a> ReleaseDocWorkflow<'a> {
    pub fn new(llm_call: LlmCall<'a>, projdir: &'a Path, task_args: &[String]) -> Self {
        Self {
            llm_call,
            projdir,
            task_args: task_args.to_vec(),
        }
    }

    fn release_range(&self) -> Result<(Option<&str>, &str), String> {
        let mut first = None;
        let mut last = "HEAD";
        let mut args = self.task_args.iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-f" | "--first" => {
                    first = Some(
                        args.next()
                            .ok_or_else(|| format!("Missing value for {arg}"))?
                            .as_str(),
                    );
                }
                "-l" | "--last" => {
                    last = args
                        .next()
                        .ok_or_else(|| format!("Missing value for {arg}"))?
                        .as_str();
                }
                _ => return Err(format!("Unknown release_doc argument: {arg}")),
            }
        }

        Ok((first, last))
    }

    fn create_context(&self) -> Result<ReleaseDocContext, crate::vc::git::GitError> {
        let git = Git::new(self.projdir);
        let (first, last) = self
            .release_range()
            .map_err(crate::vc::git::GitError::InvalidOutput)?;

        match first {
            Some(first) => ReleaseDocContext::between(&git, first, last),
            None if last == "HEAD" => ReleaseDocContext::since_latest_tag(&git),
            None => {
                let first = git.latest_tag()?.ok_or_else(|| {
                    crate::vc::git::GitError::InvalidOutput(
                        "Cannot use --last without --first when the repository has no tag".into(),
                    )
                })?;
                ReleaseDocContext::between(&git, &first, last)
            }
        }
    }
}

impl<'a> Workflow for ReleaseDocWorkflow<'a> {
    fn execute(&self) -> WorkflowResult {
        let context = match self.create_context() {
            Ok(context) => context,
            Err(err) => {
                return WorkflowResult::Error(format!(
                    "Failed to create release documentation context: {err}"
                ));
            }
        };

        self.llm_call.set_context(context.to_llm_summary_context());

        for _ in 0..self.llm_call.max_workflow_attempts() {
            let result: LlmCallResult = self.llm_call.run("");
            if result.is_done() {
                return WorkflowResult::Ok;
            }
            if !result.is_valid() {
                return WorkflowResult::LlmCallResult(result);
            }
        }

        WorkflowResult::Error(
            "Release documentation did not finish with done".into(),
        )
    }
}
