// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::generated_workflowsteps::WorkflowSteps;
use crate::runtimetools::{llmcall::LlmCall, releasedoc::ReleaseDocContext};
use crate::vc::git::Git;
use crate::workflows::workflow::{Workflow, WorkflowResult};

/// Output destination. Without -o the generated Markdown goes to stdout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseDocOutput {
    Stdout,
    File(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDocOptions {
    pub first: Option<String>,
    pub last: String,
    pub output: ReleaseDocOutput,
}

impl ReleaseDocOptions {
    pub fn parse(task_args: &[String]) -> Result<Self, String> {
        let mut first = None;
        let mut last = "HEAD".to_string();
        let mut output = ReleaseDocOutput::Stdout;
        let mut args = task_args.iter().peekable();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-f" | "--first" => {
                    first = Some(
                        args.next()
                            .filter(|next| !next.starts_with('-') && !next.is_empty())
                            .ok_or_else(|| format!("Missing value for {arg}"))?
                            .clone(),
                    );
                }
                "-l" | "--last" => {
                    last = args
                        .next()
                        .filter(|next| !next.starts_with('-') && !next.is_empty())
                        .ok_or_else(|| format!("Missing value for {arg}"))?
                        .clone();
                }
                "-o" | "--output" => {
                    let filename = args.peek().filter(|next| !next.starts_with('-')).copied();
                    let path = match filename {
                        Some(filename) => {
                            args.next();
                            if filename.is_empty() {
                                return Err("Empty filename for --output".into());
                            }
                            PathBuf::from(filename)
                        }
                        None => PathBuf::from("RELEASE_NOTES.md"),
                    };
                    output = ReleaseDocOutput::File(path);
                }
                _ => return Err(format!("Unknown release_doc argument: {arg}")),
            }
        }

        Ok(Self {
            first,
            last,
            output,
        })
    }
}

pub fn write_release_notes<W: Write>(
    output: &ReleaseDocOutput,
    projdir: &Path,
    markdown: &str,
    stdout: &mut W,
) -> io::Result<()> {
    match output {
        ReleaseDocOutput::Stdout => {
            stdout.write_all(markdown.as_bytes())?;
            stdout.flush()
        }
        ReleaseDocOutput::File(path) => {
            let target = projdir.join(path);
            fs::write(target, markdown)
        }
    }
}

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

    fn create_context(
        &self,
        options: &ReleaseDocOptions,
    ) -> Result<ReleaseDocContext, crate::vc::git::GitError> {
        let git = Git::new(self.projdir);
        let first = options.first.as_deref();
        let last = options.last.as_str();

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
        let options = match ReleaseDocOptions::parse(&self.task_args) {
            Ok(options) => options,
            Err(err) => return WorkflowResult::Error(err),
        };

        let context = match self.create_context(&options) {
            Ok(context) => context,
            Err(err) => {
                return WorkflowResult::Error(format!(
                    "Failed to create release documentation context: {err}"
                ));
            }
        };

        let analysis = match self.llm_call.run_text_step(
            WorkflowSteps::ReleaseDocAnalyze.get_prompt(),
            &context.to_llm_summary_context(),
        ) {
            Ok(analysis) => analysis,
            Err(result) => return WorkflowResult::LlmCallResult(result),
        };

        let release_notes = match self
            .llm_call
            .run_text_step(WorkflowSteps::ReleaseDocWrite.get_prompt(), &analysis)
        {
            Ok(release_notes) => release_notes,
            Err(result) => return WorkflowResult::LlmCallResult(result),
        };

        if let Err(err) = write_release_notes(
            &options.output,
            self.projdir,
            &release_notes,
            &mut io::stdout().lock(),
        ) {
            return WorkflowResult::Error(format!("Failed to write release notes: {err}"));
        }
        WorkflowResult::Ok
    }
}
