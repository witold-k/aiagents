// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Bounded, source-aware document transformation with independent review.

use std::{
    collections::BTreeMap,
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
const MAX_MERGE_CHARS: usize = 20_000;

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

const EMPTY_MARKER: &str = "NO_MESSAGE_DEFINITIONS";

/// Parse the model's explicitly delimited Markdown file fragments. No source
/// format parsing is involved: this is solely the controlled response format.
fn parse_file_records(response: &str) -> Result<Vec<(String, String)>, String> {
    if response.trim() == EMPTY_MARKER {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    let mut current_path: Option<String> = None;
    let mut content = String::new();
    for line in response.lines() {
        if let Some(path) = line.strip_prefix("=== FILE: ").and_then(|s| s.strip_suffix(" ===")) {
            if current_path.is_some() {
                return Err("Nested FILE record in model response".into());
            }
            if !valid_spec_path(path) {
                return Err(format!("Invalid specification path in model response: {path}"));
            }
            current_path = Some(path.to_owned());
            content.clear();
        } else if line == "=== END FILE ===" {
            let path = current_path.take().ok_or("END FILE without FILE")?;
            if content.trim().is_empty() {
                return Err(format!("Empty specification fragment: {path}"));
            }
            records.push((path, content.trim().to_owned()));
            content.clear();
        } else if current_path.is_some() {
            content.push_str(line);
            content.push('\n');
        } else if !line.trim().is_empty() {
            return Err("Unexpected text outside FILE records".into());
        }
    }
    if current_path.is_some() {
        return Err("Unclosed FILE record in model response".into());
    }
    if records.is_empty() {
        return Err("Model returned neither file records nor the empty marker".into());
    }
    Ok(records)
}

fn valid_spec_path(path: &str) -> bool {
    if matches!(path, "general.md" | "infrastructure.md") {
        return true;
    }
    let Some(slug) = path.strip_prefix("messages/").and_then(|s| s.strip_suffix(".md")) else {
        return false;
    };
    !slug.is_empty()
        && slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// Stage all output files in a fresh sibling directory and publish only after
/// every file has been written. Refuse to overwrite an existing specification.
fn write_spec_directory(output: &Path, files: &BTreeMap<String, String>, source: &Path) -> Result<(), String> {
    if output.exists() {
        return Err(format!("Output directory already exists: {}", output.display()));
    }
    let name = output.file_name().and_then(|s| s.to_str()).ok_or("Invalid output directory name")?;
    let stage = output.with_file_name(format!(".{name}.tmp-{}", std::process::id()));
    fs::create_dir(&stage).map_err(|e| format!("Cannot stage output: {e}"))?;
    let result = (|| {
        fs::create_dir(stage.join("messages")).map_err(|e| e.to_string())?;
        for (path, body) in files {
            fs::write(stage.join(path), format!("{body}\n")).map_err(|e| format!("Cannot write {path}: {e}"))?;
        }
        let mut readme = format!("# Extracted Message Specifications\n\nSource: `{}`\n\n", source.display());
        readme.push_str("## Shared specifications\n\n- [General protocol rules](general.md)\n- [Infrastructure contracts](infrastructure.md)\n\n## Messages\n\n");
        for path in files.keys().filter(|p| p.starts_with("messages/")) {
            readme.push_str(&format!("- [{path}]({path})\n"));
        }
        readme.push_str("\n## Verification status\n\nEach fragment was reviewed against its source window. Cross-window reconciliation and unresolved conflicts require further review.\n");
        fs::write(stage.join("README.md"), readme).map_err(|e| e.to_string())?;
        fs::rename(&stage, output).map_err(|e| format!("Cannot publish output: {e}"))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
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
        let output = input.with_file_name(format!("{stem}_messages"));
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
        let merge_prompt = format!(
            "{requirements}\n\n{}",
            WorkflowSteps::ChunkedTransformMerge.get_prompt()
        );
        let mut files: BTreeMap<String, String> = BTreeMap::new();
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
                &extract_prompt, &context, MAX_TOKENS, 0.2,
            ) {
                Ok(text) => text,
                Err(err) => return WorkflowResult::LlmCallResult(err),
            };
            let review_context = format!("{context}\n\n=== EXTRACTED DRAFT ===\n{draft}");
            println!("## [CHUNKED] VALIDATE {}/{}", index + 1, parts.len());
            let checked = match self.llm_call.run_text_step_limited_with_temperature(
                &validate_prompt, &review_context, MAX_TOKENS, 0.1,
            ) {
                Ok(text) => text,
                Err(err) => return WorkflowResult::LlmCallResult(err),
            };
            let records = match parse_file_records(&checked) {
                Ok(records) => records,
                Err(err) => return WorkflowResult::Error(format!("Chunk {}: {err}", index + 1)),
            };
            for (path, fragment) in records {
                if let Some(existing) = files.get(&path) {
                    let merge_context = format!(
                        "DESTINATION: {path}\n\n=== EXISTING SPECIFICATION ===\n{existing}\n\n=== NEW REVIEWED FRAGMENT ===\n{fragment}"
                    );
                    if merge_context.chars().count() > MAX_MERGE_CHARS {
                        return WorkflowResult::Error(format!(
                            "Cannot safely merge {path}: context exceeds {MAX_MERGE_CHARS} characters"
                        ));
                    }
                    println!("## [CHUNKED] MERGE {path}");
                    let merged = match self.llm_call.run_text_step_limited_with_temperature(
                        &merge_prompt, &merge_context, MAX_TOKENS, 0.1,
                    ) {
                        Ok(text) => text,
                        Err(err) => return WorkflowResult::LlmCallResult(err),
                    };
                    if merged.trim().is_empty() {
                        return WorkflowResult::Error(format!("Empty merge result for {path}"));
                    }
                    files.insert(path, merged);
                } else {
                    files.insert(path, fragment);
                }
            }
        }

        if files.is_empty() {
            return WorkflowResult::Error("No protocol specifications were extracted".into());
        }
        files.entry("general.md".into()).or_insert_with(|| "# General Protocol Rules\n\nNo shared rules were established from the source.".into());
        files.entry("infrastructure.md".into()).or_insert_with(|| "# Infrastructure Contracts\n\nNo helper contracts were established from the source.".into());
        match write_spec_directory(&output, &files, &input) {
            Ok(()) => {
                println!("## [CHUNKED] WROTE {}", output.display());
                WorkflowResult::Ok
            }
            Err(err) => WorkflowResult::Error(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CHUNK_OVERLAP_CHARS, chunks, parse_file_records, valid_spec_path};

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
            assert_eq!(
                tail,
                pair[1]
                    .text
                    .chars()
                    .take(CHUNK_OVERLAP_CHARS)
                    .collect::<String>()
            );
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
        let source = format!(
            "![figure](data:image/png;base64,{})\nNext",
            "A".repeat(50_000)
        );
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
        assert_eq!(
            parts.iter().map(|p| p.text.as_str()).collect::<String>(),
            "abc"
        );
    }
    #[test]
    fn parses_multiple_specification_files() {
        let response = "=== FILE: general.md ===\n# General\n=== END FILE ===\n=== FILE: messages/a-1.md ===\n# A\n=== END FILE ===";
        let records = parse_file_records(response).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].0, "messages/a-1.md");
    }

    #[test]
    fn rejects_unsafe_or_malformed_model_paths() {
        assert!(!valid_spec_path("messages/../escape.md"));
        assert!(!valid_spec_path("README.md"));
        assert!(!valid_spec_path("messages/A.md"));
        assert!(parse_file_records("=== FILE: ../oops.md ===\nfoo\n=== END FILE ===").is_err());
        assert!(parse_file_records("garbage").is_err());
        assert!(parse_file_records("=== FILE: general.md ===\nfoo").is_err());
    }

}
