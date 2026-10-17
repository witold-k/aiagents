// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Bounded, source-aware document transformation with independent review.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::generated_tasks::Tasks;
use crate::generated_workflowsteps::WorkflowSteps;
use crate::runtimetools::llmcall::LlmCall;
use crate::workflows::workflow::{Workflow, WorkflowResult};

const CHUNK_CHARS: usize = 12_000;
const MAX_TOKENS: u32 = 12_000;

#[derive(Debug, PartialEq, Eq)]
struct Chunk {
    start_line: usize,
    end_line: usize,
    text: String,
}

/// Split on paragraph boundaries where possible. Oversized blocks are split
/// on line boundaries; every source line is included exactly once.
fn chunks(source: &str, limit: usize) -> Vec<Chunk> {
    if source.is_empty() {
        return Vec::new();
    }
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut result = Vec::new();
    let mut start = 0;
    while start < lines.len() {
        let mut end = start;
        let mut size = 0;
        let mut last_blank = None;
        while end < lines.len() {
            let len = lines[end].len();
            if end > start && size + len > limit {
                break;
            }
            size += len;
            end += 1;
            if lines[end - 1].trim().is_empty() {
                last_blank = Some(end);
            }
            if size >= limit {
                break;
            }
        }
        if end < lines.len()
            && let Some(blank) = last_blank
            && blank > start
        {
            end = blank;
        }
        // An individual line may exceed the limit, but is never truncated.
        if end == start {
            end += 1;
        }
        result.push(Chunk {
            start_line: start + 1,
            end_line: end,
            text: lines[start..end].concat(),
        });
        start = end;
    }
    result
}

pub struct ChunkedTransformWorkflow<'a> {
    llm_call: LlmCall<'a>,
    projdir: &'a Path,
    task_args: Vec<String>,
}

impl<'a> ChunkedTransformWorkflow<'a> {
    pub fn new(llm_call: LlmCall<'a>, projdir: &'a Path, task_args: &[String]) -> Self {
        Self {
            llm_call,
            projdir,
            task_args: task_args.to_vec(),
        }
    }

    fn paths(&self) -> Result<(PathBuf, PathBuf), String> {
        if self.task_args.len() != 1 {
            return Err(
                "extract_message_tables requires exactly one input .md or .txt path after -a"
                    .into(),
            );
        }
        let path = self.projdir.join(&self.task_args[0]);
        let input = path
            .canonicalize()
            .map_err(|err| format!("Cannot open {}: {err}", path.display()))?;
        let root = self.projdir.canonicalize().map_err(|err| err.to_string())?;
        if !input.starts_with(&root) {
            return Err("Input must be inside the project directory".into());
        }
        if !matches!(
            input.extension().and_then(|ext| ext.to_str()),
            Some("md" | "txt")
        ) {
            return Err("Input must be a .md or .txt file".into());
        }
        let stem = input
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or("Input filename is not valid UTF-8")?;
        let output = input.with_file_name(format!("{stem}_messages.md"));
        if output == input {
            return Err("Output would overwrite input".into());
        }
        Ok((input, output))
    }
}

impl Workflow for ChunkedTransformWorkflow<'_> {
    fn execute(&self) -> WorkflowResult {
        let (input, output) = match self.paths() {
            Ok(paths) => paths,
            Err(err) => return WorkflowResult::Error(err),
        };
        let source = match fs::read_to_string(&input) {
            Ok(source) => source,
            Err(err) => {
                return WorkflowResult::Error(format!("Cannot read {}: {err}", input.display()));
            }
        };
        let parts = chunks(&source, CHUNK_CHARS);
        if parts.is_empty() {
            return WorkflowResult::Error("Input document is empty".into());
        }

        // Each review receives the original source excerpt and the draft,
        // never the entire document or an unbounded collection of drafts.
        let requirements = Tasks::ExtractMessageTables.get_prompt();
        let extract_prompt = format!(
            "{requirements}\n\n{}",
            WorkflowSteps::ChunkedTransformExtract.get_prompt()
        );
        let validate_prompt = format!(
            "{requirements}\n\n{}",
            WorkflowSteps::ChunkedTransformValidate.get_prompt()
        );
        let mut sections = Vec::with_capacity(parts.len());
        for (index, part) in parts.iter().enumerate() {
            println!(
                "## [CHUNKED] EXTRACT {}/{} (lines {}-{})",
                index + 1,
                parts.len(),
                part.start_line,
                part.end_line
            );
            let context = format!(
                "SOURCE: {}\nCHUNK: {}/{}\nLINES: {}-{}\n\n=== SOURCE ===\n{}",
                input.display(),
                index + 1,
                parts.len(),
                part.start_line,
                part.end_line,
                part.text
            );
            let draft = match self.llm_call.run_text_step_limited_with_temperature(
                &extract_prompt,
                &context,
                MAX_TOKENS,
                0.2,
            ) {
                Ok(text) => text,
                Err(err) => return WorkflowResult::LlmCallResult(err),
            };
            let review_context = format!("{context}\n\n=== EXTRACTED DRAFT ===\n{draft}");
            println!("## [CHUNKED] VALIDATE {}/{}", index + 1, parts.len());
            let checked = match self.llm_call.run_text_step_limited_with_temperature(
                &validate_prompt,
                &review_context,
                MAX_TOKENS,
                0.1,
            ) {
                Ok(text) => text,
                Err(err) => return WorkflowResult::LlmCallResult(err),
            };
            if checked.trim() == "NO_MESSAGE_DEFINITIONS" {
                continue;
            }
            sections.push(format!(
                "## Source lines {}–{}\n\n{}",
                part.start_line,
                part.end_line,
                checked.trim()
            ));
        }

        // Assemble deterministically: never pass the complete result back to
        // the LLM, because it may exceed the context window.
        let document = format!(
            "# Extracted Message Specifications\n\nSource: `{}`\n\n\
             Each section was extracted and checked against its source range.\n\n\
             Cross-section references and unresolved ambiguities require review.\n\n{}\n",
            input.display(),
            sections.join("\n\n")
        );
        match fs::write(&output, document) {
            Ok(()) => {
                println!("## [CHUNKED] WROTE {}", output.display());
                WorkflowResult::Ok
            }
            Err(err) => WorkflowResult::Error(format!("Cannot write {}: {err}", output.display())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::chunks;

    #[test]
    fn preserves_every_line_once() {
        let source = "a\nb\n\nc\nd\n\ne";
        let parts = chunks(source, 5);
        assert_eq!(
            parts.iter().map(|c| c.text.as_str()).collect::<String>(),
            source
        );
        assert_eq!(parts.first().unwrap().start_line, 1);
        assert_eq!(parts.last().unwrap().end_line, 7);
    }

    #[test]
    fn oversized_line_is_not_truncated() {
        let source = format!("{}\nnext", "x".repeat(100));
        let parts = chunks(&source, 10);
        assert_eq!(
            parts.iter().map(|c| c.text.as_str()).collect::<String>(),
            source
        );
    }

    #[test]
    fn empty_document_has_no_chunks() {
        assert!(chunks("", 100).is_empty());
    }
}
