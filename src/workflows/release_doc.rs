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

/// Split at line boundaries; a single exceptionally long line remains intact.
pub fn split_diff(diff: &str, max_chars: usize) -> Vec<String> {
    assert!(max_chars > 0);
    if diff.is_empty() {
        return vec![String::new()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in diff.split_inclusive('\n') {
        if !current.is_empty() && current.len() + line.len() > max_chars {
            chunks.push(std::mem::take(&mut current));
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// Optional diagnostic capture. Never changes LLM inputs or the generated document.
fn trace_write(dir: &Path, repository: &str, filename: &str, content: &str) {
    if let Err(error) = fs::write(
        dir.join(format!("release_doc_{repository}_{filename}")),
        content,
    ) {
        eprintln!("release_doc trace: could not write {filename}: {error}");
    }
}

fn reduce_summaries(
    llm: &LlmCall<'_>,
    mut summaries: Vec<String>,
    trace: Option<(&Path, &str, String)>,
) -> Result<String, crate::runtimetools::llmcall::LlmCallResult> {
    if summaries.is_empty() {
        return Ok("No changed files.".into());
    }
    // Bounded fan-in ensures that even a large release is processed incrementally.
    let mut level = 0;
    while summaries.len() > 1 {
        let mut next = Vec::new();
        for batch in summaries.chunks(4) {
            let prompt = "Consolidate these release change analyses without losing technical evidence. Preserve verified behavioral changes, control flow, error handling, fallible operations, dependencies, tests, file/symbol references and uncertainty. Group related changes across files, but do not replace concrete mechanisms with generic quality claims. Keep technically relevant internal details for the critical review even if not release-worthy. Do not invent risks or assert tests passed. Return factual Markdown analysis, not release notes.";
            next.push(llm.run_text_step(prompt, &batch.join("\n\n--- ANALYSIS ---\n\n"))?);
        }
        if let Some((dir, repository, ref prefix)) = trace {
            for (index, summary) in next.iter().enumerate() {
                trace_write(
                    dir,
                    repository,
                    &format!("{prefix}-level-{level:02}-batch-{index:03}.md"),
                    summary,
                );
            }
        }
        summaries = next;
        level += 1;
    }
    Ok(summaries.pop().unwrap())
}

/// Build review input from original Git diffs, not LLM-generated summaries.
/// Only complete per-file diffs are included; omitted files are listed explicitly.
pub fn build_review_input(
    metadata: &str,
    changes: &[crate::runtimetools::releasedoc::ReleaseFileChange],
) -> String {
    const MAX_DIFF_BYTES: usize = 64_000;
    let mut input = format!("{metadata}\n=== ORIGINAL GIT DIFFS ===\n");
    let mut used = 0;
    let mut omitted = Vec::new();
    let mut ordered: Vec<_> = changes.iter().collect();
    ordered.sort_by_key(|change| {
        let path = change.path.to_string_lossy();
        if path.starts_with("src/") || path == "build.rs" {
            0
        } else if path.starts_with("tests/") {
            1
        } else {
            2
        }
    });
    for change in ordered {
        let path = change.path.display().to_string();
        let entry = format!("\n=== FILE: {path} ===\n{}\n", change.diff);
        if entry.len() <= MAX_DIFF_BYTES.saturating_sub(used) {
            used += entry.len();
            input.push_str(&entry);
        } else {
            omitted.push(path);
        }
    }
    if !omitted.is_empty() {
        input.push_str("\n=== OMITTED FILE DIFFS (NOT REVIEWED) ===\n");
        for path in omitted {
            input.push_str(&format!("- {path}\n"));
        }
        input.push_str("Do not infer behavior from omitted diffs.\n");
    }
    input
}
/// Only an exact OK response skips correction; ambiguous validator output is not silently accepted.
pub fn review_validation_is_ok(response: &str) -> bool {
    response.trim() == "OK"
}

/// Keep original diffs, the draft, and the critique in the correction context.
pub fn build_review_validation_input(review_input: &str, review: &str) -> String {
    format!("{review_input}\n=== CURRENT TECHNICAL REVIEW ===\n{review}\n")
}

pub fn build_review_revision_input(validation_input: &str, feedback: &str) -> String {
    format!("{validation_input}\n=== VALIDATION FEEDBACK ===\n{feedback}\n")
}

/// Remove only a full-document Markdown fence, leaving ordinary Markdown untouched.
fn strip_markdown_fence(markdown: &str) -> &str {
    let text = markdown.trim();
    let Some(first_line_end) = text.find('\n') else {
        return text;
    };
    let first_line = text[..first_line_end].trim();
    if first_line != "```" && first_line != "```markdown" && first_line != "```md" {
        return text;
    }
    let body = text[first_line_end + 1..].trim_end();
    body.strip_suffix("```").map(str::trim_end).unwrap_or(text)
}

/// Append review and the complete Git-derived path list without asking the LLM to invent paths.
pub fn assemble_release_document(
    release_notes: &str,
    review: &str,
    changes: &[crate::runtimetools::releasedoc::ReleaseFileChange],
) -> String {
    let mut document = strip_markdown_fence(release_notes).trim_end().to_owned();
    document.push_str("\n\n## Technical Review\n\n");
    let review = strip_markdown_fence(review);
    let review = review.trim_start();
    let review = review
        .strip_prefix("## Technical Review")
        .filter(|rest| rest.is_empty() || rest.starts_with('\n') || rest.starts_with('\r'))
        .unwrap_or(review);
    document.push_str(review.trim());
    document.push_str("\n\n## Changed Files\n\n");
    if changes.is_empty() {
        document.push_str("No changed files.\n");
    } else {
        let mut paths: Vec<_> = changes.iter().map(|change| &change.path).collect();
        paths.sort();
        paths.dedup();
        for path in paths {
            // Keep unusual file names on a single Markdown line.
            let name = path
                .to_string_lossy()
                .replace('\n', "\\n")
                .replace('\r', "\\r");
            document.push_str(&format!("- `{}`\n", name.replace('`', "\\`")));
        }
    }
    document
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

        // The checkout directory names the repository, without any remote URL.
        let repository = self
            .projdir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("repository")
            .trim_end_matches(".git");
        let repository = if repository.is_empty() {
            "repository"
        } else {
            repository
        };
        let repository: String = repository
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();

        // Opt-in diagnostic artifacts for locating information loss between stages.
        // Contents may include source code and commit messages; keep the directory private.
        let trace_dir = std::env::var_os("AIFIX_RELEASE_DOC_TRACE_DIR").map(|value| {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                self.projdir.join(path)
            }
        });
        if let Some(dir) = &trace_dir {
            if let Err(error) = fs::create_dir_all(dir) {
                return WorkflowResult::Error(format!(
                    "Failed to create release_doc trace directory: {error}"
                ));
            }
            trace_write(
                dir,
                &repository,
                "00-release-metadata.txt",
                &context.release_metadata(),
            );
        }

        // Analyze the net diff of each changed file in sequence. Do not pass
        // the entire repository or a combined release diff to the model.
        let mut file_summaries = Vec::new();
        for (file_index, change) in context.changes.iter().enumerate() {
            let chunks = split_diff(&change.diff, 16_000);
            let mut chunk_summaries = Vec::new();
            for (index, chunk) in chunks.iter().enumerate() {
                let input = format!(
                    "FILE: {}\nPART: {}/{}\nNET DIFF:\n{}",
                    change.path.display(),
                    index + 1,
                    chunks.len(),
                    chunk,
                );
                let prompt = format!(
                    "{}\nAnalyze only this file diff. Report concise, evidence-grounded changes,                      including uncertainty. Do not write release notes yet.",
                    WorkflowSteps::ReleaseDocAnalyze.get_prompt(),
                );
                let summary = match self.llm_call.run_text_step(&prompt, &input) {
                    Ok(summary) => summary,
                    Err(result) => return WorkflowResult::LlmCallResult(result),
                };
                if let Some(dir) = &trace_dir {
                    trace_write(
                        dir,
                        &repository,
                        &format!("01-file-{file_index:03}-chunk-{index:03}.md"),
                        &summary,
                    );
                }
                chunk_summaries.push(summary);
            }
            let summary = match reduce_summaries(
                &self.llm_call,
                chunk_summaries,
                trace_dir
                    .as_deref()
                    .map(|dir| (dir, repository.as_str(), format!("02-file-{file_index:03}"))),
            ) {
                Ok(summary) => summary,
                Err(result) => return WorkflowResult::LlmCallResult(result),
            };
            let file_summary = format!("FILE: {}\n{}", change.path.display(), summary);
            if let Some(dir) = &trace_dir {
                trace_write(
                    dir,
                    &repository,
                    &format!("03-file-{file_index:03}-summary.md"),
                    &file_summary,
                );
            }
            file_summaries.push(file_summary);
        }

        // Merge related changes across files before writing the final document.
        let combined = match reduce_summaries(
            &self.llm_call,
            file_summaries,
            trace_dir
                .as_deref()
                .map(|dir| (dir, repository.as_str(), "04-cross-file".to_string())),
        ) {
            Ok(summary) => summary,
            Err(result) => return WorkflowResult::LlmCallResult(result),
        };
        if let Some(dir) = &trace_dir {
            trace_write(dir, &repository, "05-consolidated-analysis.md", &combined);
        }
        let input = format!(
            "{}\n=== VERIFIED FILE ANALYSES ===\n{}",
            context.release_metadata(),
            combined,
        );
        let release_notes = match self
            .llm_call
            .run_text_step(WorkflowSteps::ReleaseDocWrite.get_prompt(), &input)
        {
            Ok(notes) => notes,
            Err(result) => return WorkflowResult::LlmCallResult(result),
        };

        if let Some(dir) = &trace_dir {
            trace_write(dir, &repository, "06-release-notes.md", &release_notes);
        }

        // Review original Git evidence, independently of LLM reductions.
        let review_input = build_review_input(&context.release_metadata(), &context.changes);
        if let Some(dir) = &trace_dir {
            trace_write(dir, &repository, "07-review-input.txt", &review_input);
        }
        // A failed review must not discard already generated release notes.
        // This can happen with large inputs (e.g. binary diffs or token limits),
        // but also with unrelated LLM failures. Never imply a successful review.
        let final_review = match self
            .llm_call
            .run_text_step(WorkflowSteps::ReleaseDocReview.get_prompt(), &review_input)
        {
            Err(error) => {
                eprintln!("release_doc: technical review failed: {error}");
                "Technical review unavailable: the review LLM step failed. Release notes have not been independently reviewed; consult the original Git diffs.".to_string()
            }
            Ok(review) => {
                if let Some(dir) = &trace_dir {
                    trace_write(dir, &repository, "07-technical-review.md", &review);
                }
                let validation_input = build_review_validation_input(&review_input, &review);
                match self.llm_call.run_text_step(
                    WorkflowSteps::ReleaseDocValidate.get_prompt(),
                    &validation_input,
                ) {
                    Err(error) => {
                        eprintln!("release_doc: technical review validation failed: {error}");
                        format!(
                            "{review}\n\n**Review status: NOT VALIDATED.** The validation LLM step failed; the technical review may be incomplete or incorrect."
                        )
                    }
                    Ok(feedback) => {
                        if let Some(dir) = &trace_dir {
                            trace_write(dir, &repository, "08-review-validation.md", &feedback);
                        }
                        if review_validation_is_ok(&feedback) {
                            review
                        } else {
                            let revision_input =
                                build_review_revision_input(&validation_input, &feedback);
                            match self.llm_call.run_text_step(
                                WorkflowSteps::ReleaseDocRevise.get_prompt(),
                                &revision_input,
                            ) {
                                Ok(revised) => {
                                    if let Some(dir) = &trace_dir {
                                        trace_write(
                                            dir,
                                            &repository,
                                            "09-revised-review.md",
                                            &revised,
                                        );
                                    }
                                    format!(
                                        "{revised}\n\n**Review status: REVISED, NOT REVALIDATED.**"
                                    )
                                }
                                Err(error) => {
                                    eprintln!(
                                        "release_doc: technical review revision failed: {error}"
                                    );
                                    format!(
                                        "{review}\n\n**Review status: VALIDATION FAILED; REVISION UNAVAILABLE.** The revision LLM step failed. Validation feedback:\n\n{feedback}"
                                    )
                                }
                            }
                        }
                    }
                }
            }
        };

        let document = assemble_release_document(&release_notes, &final_review, &context.changes);
        if let Some(dir) = &trace_dir {
            trace_write(dir, &repository, "10-final-document.md", &document);
        }

        if let Err(err) = write_release_notes(
            &options.output,
            self.projdir,
            &document,
            &mut io::stdout().lock(),
        ) {
            return WorkflowResult::Error(format!("Failed to write release notes: {err}"));
        }
        WorkflowResult::Ok
    }
}
