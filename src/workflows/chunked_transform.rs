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
const CHUNK_OVERLAP_CHARS: usize = 1_000;
const MAX_TOKENS: u32 = 10_000;

#[derive(Debug, PartialEq, Eq)]
struct Chunk {
    start_line: usize,
    end_line: usize,
    text: String,
}

/// Create overlapping character windows without interpreting the source format.
/// All boundaries are valid UTF-8 boundaries; the final window is not padded.
/// Line ranges are informational, and can overlap between adjacent windows.
fn chunks(source: &str, limit: usize) -> Vec<Chunk> {
    if source.is_empty() || limit == 0 {
        return Vec::new();
    }

    // For tiny test budgets, preserve forward progress as well.
    let overlap = CHUNK_OVERLAP_CHARS.min(limit.saturating_sub(1));
    let offsets: Vec<usize> = source
        .char_indices()
        .map(|(byte_offset, _)| byte_offset)
        .chain(std::iter::once(source.len()))
        .collect();
    let character_count = offsets.len() - 1;
    let mut result = Vec::new();
    let mut start = 0;

    loop {
        let end = (start + limit).min(character_count);
        let start_byte = offsets[start];
        let end_byte = offsets[end];
        let start_line = 1 + source[..start_byte]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count();
        let text = &source[start_byte..end_byte];
        let newlines = text.bytes().filter(|byte| *byte == b'\n').count();
        let end_line = start_line + newlines - usize::from(text.ends_with('\n'));
        result.push(Chunk {
            start_line,
            end_line,
            text: text.to_owned(),
        });
        if end == character_count {
            break;
        }
        start = end - overlap;
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
                "SOURCE: {}\nCHUNK: {}/{}\nLINES: {}-{}\nADJACENT CHUNKS OVERLAP BY UP TO {} CHARACTERS. Do not count repeated source facts twice.\n\n=== SOURCE ===\n{}",
                input.display(),
                index + 1,
                parts.len(),
                part.start_line,
                part.end_line,
                CHUNK_OVERLAP_CHARS,
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
    use super::{chunks, CHUNK_OVERLAP_CHARS};

    #[test]
    fn short_document_is_unchanged() {
        let source = "a\nb\n\nc\nd\n\ne";
        let parts = chunks(source, 100);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].text, source);
        assert_eq!(parts[0].start_line, 1);
        assert_eq!(parts[0].end_line, 7);
    }

    #[test]
    fn adjacent_chunks_overlap_exactly() {
        let source = "x".repeat(3000);
        let parts = chunks(&source, 1200);
        assert!(parts.len() > 2);
        for pair in parts.windows(2) {
            assert_eq!(pair[0].text.chars().count(), 1200);
            let tail: String = pair[0].text.chars().skip(200).collect();
            assert_eq!(tail, pair[1].text.chars().take(CHUNK_OVERLAP_CHARS).collect::<String>());
        }
        assert!(parts.iter().all(|part| part.text.chars().count() <= 1200));
    }

    #[test]
    fn unicode_and_newlines_preserve_source_with_overlap() {
        let source = "😀é\n".repeat(2000);
        let parts = chunks(&source, 1100);
        let mut recovered = parts[0].text.clone();
        for part in parts.iter().skip(1) {
            recovered.extend(part.text.chars().skip(CHUNK_OVERLAP_CHARS));
        }
        assert_eq!(recovered, source);
        assert!(parts.iter().all(|part| part.text.chars().count() <= 1100));
    }

    #[test]
    fn embedded_image_data_remains_bounded() {
        let source = format!("![figure](data:image/png;base64,{})\nNext", "A".repeat(50_000));
        let parts = chunks(&source, 12_000);
        assert!(parts.len() >= 5);
        assert!(parts.iter().all(|part| part.text.chars().count() <= 12_000));
    }

    #[test]
    fn empty_document_has_no_chunks() {
        assert!(chunks("", 100).is_empty());
        assert!(chunks("a", 0).is_empty());
    }

    #[test]
    fn one_character_windows_make_progress() {
        let parts = chunks("abc", 1);
        assert_eq!(parts.iter().map(|p| p.text.as_str()).collect::<String>(), "abc");
    }
}
