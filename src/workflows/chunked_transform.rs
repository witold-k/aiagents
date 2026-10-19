// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Bounded, source-aware document transformation with independent review.

use serde::{Deserialize, Serialize};

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::generated_tasks::Tasks;
use crate::generated_workflowsteps::WorkflowSteps;
use crate::runtimetools::llmcall::LlmCall;
use crate::utils::stringutils::strip_code_fences;
use crate::workflows::workflow::{Workflow, WorkflowResult};

const CHUNK_CHARS: usize = 4_000;
pub const CHUNK_OVERLAP_CHARS: usize = 1_000;
const MAX_TOKENS: u32 = 2_048;
const MAX_MERGE_CHARS: usize = 20_000;
const MAX_AUDIT_PASSES: usize = 2;
const AUDIT_VERIFICATION_PASS: usize = MAX_AUDIT_PASSES;

const CHECKPOINT_VERSION: u32 = 5;

#[derive(Debug, Serialize, Deserialize)]
struct ExtractionCheckpoint {
    version: u32,
    source_fingerprint: String,
    next_chunk: usize,
    files: BTreeMap<String, String>,
    #[serde(default)]
    source_windows: BTreeMap<String, Vec<usize>>,
    #[serde(default)]
    coverage_next_chunk: usize,
    #[serde(default)]
    audit: Option<AuditCheckpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditCheckpoint {
    prompt_fingerprint: String,
    pass: usize,
    file_index: usize,
    evidence_index: usize,
    changed: bool,
    #[serde(default)]
    masked_evidence: bool,
}

/// Stable fingerprint of the source and the prompt/configuration affecting extraction.
/// This detects accidental changes between runs without requiring another dependency.
fn extraction_fingerprint(source: &str, prompts: &[&str]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for bytes in std::iter::once(source.as_bytes()).chain(prompts.iter().map(|p| p.as_bytes())) {
        for byte in (bytes.len() as u64)
            .to_le_bytes()
            .iter()
            .chain(bytes.iter())
        {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    format!("{hash:016x}")
}

fn checkpoint_path(output: &Path) -> PathBuf {
    output.with_extension("v5.checkpoint.json")
}

/// Atomically replace a complete checkpoint; never save a half-merged chunk.
fn save_checkpoint(path: &Path, checkpoint: &ExtractionCheckpoint) -> Result<(), String> {
    let tmp = path.with_extension("checkpoint.tmp");
    let content = serde_json::to_vec_pretty(checkpoint)
        .map_err(|err| format!("Cannot serialize extraction checkpoint: {err}"))?;
    fs::write(&tmp, content).map_err(|err| format!("Cannot write checkpoint: {err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("Cannot publish checkpoint: {err}"))
}

fn load_checkpoint(
    path: &Path,
    fingerprint: &str,
    total_chunks: usize,
) -> Result<Option<ExtractionCheckpoint>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read(path).map_err(|err| format!("Cannot read checkpoint: {err}"))?;
    let checkpoint: ExtractionCheckpoint = serde_json::from_slice(&data)
        .map_err(|err| format!("Invalid extraction checkpoint: {err}"))?;
    if checkpoint.version != CHECKPOINT_VERSION
        || checkpoint.source_fingerprint != fingerprint
        || checkpoint.next_chunk > total_chunks
        || checkpoint.coverage_next_chunk > checkpoint.next_chunk
        || checkpoint.files.keys().any(|path| !valid_spec_path(path))
        || checkpoint.source_windows.iter().any(|(path, indices)| {
            !valid_spec_path(path) || indices.iter().any(|index| *index >= total_chunks)
        })
        || checkpoint.audit.as_ref().is_some_and(|audit| {
            checkpoint.next_chunk != total_chunks
                || audit.pass > AUDIT_VERIFICATION_PASS
                || audit.file_index > checkpoint.files.len()
        })
    {
        return Err(format!(
            "Checkpoint {} does not match the current source or workflow; move or delete it to restart",
            path.display()
        ));
    }
    Ok(Some(checkpoint))
}

/// Normalize a model's audit decision. The extraction empty marker means
/// that the evidence supplies no message definitions requiring a revision.
pub fn parse_audit_decision(response: &str) -> Result<bool, String> {
    let decision = response.trim();
    match decision {
        "UNCHANGED" | "NO_MESSAGE_DEFINITIONS" | "=== NO_MESSAGE_DEFINITIONS ===" => Ok(false),
        "REVISE" => Ok(true),
        _ => Err(format!("Invalid audit decision: {decision}")),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Chunk {
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
}

/// Create overlapping character windows without interpreting the source format.
/// All boundaries are valid UTF-8 boundaries; the final window is not padded.
/// Line ranges are informational, and can overlap between adjacent windows.
pub fn chunks(source: &str, limit: usize) -> Vec<Chunk> {
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

/// Mask long encoded runs in Markdown data URIs, without interpreting the
/// protocol source. Preserve newlines so source line references remain valid.
/// This only targets explicit data URIs, never arbitrary base64-looking fields.
pub fn mask_embedded_data_uris(text: &str) -> String {
    let mut result = String::with_capacity(text.len().min(8_192));
    let mut rest = text;
    while let Some(start) = rest.find("data:image/") {
        result.push_str(&rest[..start]);
        let candidate = &rest[start..];
        let Some(marker) = candidate.find(";base64,") else {
            result.push_str("data:image/");
            rest = &candidate["data:image/".len()..];
            continue;
        };
        if marker > 100 || candidate[..marker].contains(['\n', '\r', ' ', '"', '\'']) {
            result.push_str("data:image/");
            rest = &candidate["data:image/".len()..];
            continue;
        }
        let payload_start = marker + ";base64,".len();
        let bytes = candidate.as_bytes();
        let mut end = payload_start;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric()
                || matches!(bytes[end], b'+' | b'/' | b'=' | b'_' | b'-'))
        {
            end += 1;
        }
        if end - payload_start < 128 {
            result.push_str(&candidate[..end]);
        } else {
            result.push_str(&candidate[..payload_start]);
            for byte in &bytes[payload_start..end] {
                result.push(if *byte == b'\n' { '\n' } else { ' ' });
            }
        }
        rest = &candidate[end..];
    }
    result.push_str(rest);
    result
}

/// Remove opaque spans from model evidence while preserving character and line
/// positions. This is deliberately conservative: only very long lines with
/// almost no word separators are considered encoded data.
pub fn mask_opaque_lines(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut run = String::new();
    let flush = |run: &mut String, output: &mut String| {
        if run.chars().count() >= 512 {
            output.extend(std::iter::repeat_n(' ', run.chars().count()));
        } else {
            output.push_str(run);
        }
        run.clear();
    };
    for character in source.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '=' | '_' | '-') {
            run.push(character);
        } else {
            flush(&mut run, &mut result);
            result.push(character);
        }
    }
    flush(&mut run, &mut result);
    result
}

/// Coverage may only discover message specifications, never rewrite shared files.
pub fn coverage_message_path(path: &str) -> bool {
    path.starts_with("messages/") && valid_spec_path(path)
}

/// Preserve every reviewed fragment without asking a model to reproduce the
/// previous specification. Repeated identical fragments are skipped. Other
/// overlap is kept explicitly for later source-grounded reconciliation.
pub fn append_reviewed_fragment(existing: &str, fragment: &str) -> String {
    let new_fragment = fragment.trim();
    if new_fragment.is_empty() || existing.trim() == new_fragment || existing.contains(new_fragment)
    {
        return existing.to_owned();
    }
    format!("{existing}\n\n---\n\n{new_fragment}")
}

/// A merge may add reviewed facts, but must not silently rewrite existing facts.
pub fn merge_preserves_existing(existing: &str, merged: &str) -> bool {
    !merged.trim().is_empty() && merged.contains(existing.trim())
}

/// A narrowly scoped audit edit; the quote must occur in the supplied evidence.
#[derive(Debug, Deserialize)]
struct AuditEdit {
    evidence_quote: String,
    old_text: String,
    new_text: String,
}

pub fn apply_audit_edit(existing: &str, evidence: &str, response: &str) -> Result<String, String> {
    let trimmed = response.trim();
    // Accept only a complete outer fence. The shared filter also extracts
    // embedded JSON blocks from prose, which is too permissive for audit edits.
    if trimmed.starts_with("```") {
        if !trimmed.ends_with("```") {
            return Err("Unclosed audit edit JSON fence or trailing commentary".into());
        }
        let Some((opening, body)) = trimmed.split_once('\n') else {
            return Err("Missing audit edit JSON body".into());
        };
        if opening != "```json" && opening != "```" {
            return Err("Unexpected audit edit fence language".into());
        }
        let inner = body
            .strip_suffix("```")
            .expect("closing fence checked above");
        if inner.contains("```") {
            return Err("Nested audit edit fences are not allowed".into());
        }
    } else if trimmed.contains("```") {
        return Err("Audit edit contains a non-outer code fence".into());
    }
    let normalized = strip_code_fences(trimmed);
    let edit: AuditEdit = serde_json::from_str(&normalized)
        .map_err(|err| format!("Invalid audit edit JSON: {err}"))?;
    let quote = edit.evidence_quote.trim();
    if quote.chars().count() < 12 || !evidence.contains(quote) {
        return Err("Audit edit has no substantial verbatim quote in evidence".into());
    }
    if edit.old_text.is_empty() || edit.new_text.is_empty() {
        return Err("Audit edit must replace nonempty text with nonempty text".into());
    }
    if existing.matches(&edit.old_text).count() != 1 {
        return Err("Audit edit old_text must occur exactly once in specification".into());
    }
    if edit.old_text == edit.new_text {
        return Err("Audit edit makes no change".into());
    }
    Ok(existing.replacen(&edit.old_text, &edit.new_text, 1))
}

const EMPTY_MARKER: &str = "NO_MESSAGE_DEFINITIONS";

/// Parse the model's explicitly delimited Markdown file fragments. No source
/// format parsing is involved: this is solely the controlled response format.
pub fn parse_file_records(response: &str) -> Result<Vec<(String, String)>, String> {
    let trimmed = response.trim();
    if matches!(trimmed, EMPTY_MARKER | "=== NO_MESSAGE_DEFINITIONS ===") {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    let mut current_path: Option<String> = None;
    let mut content = String::new();
    let mut fenced = false;
    for line in response.lines() {
        let line = line.trim_end();
        let stripped = line.trim();
        // Models sometimes wrap each Markdown record body in a code fence.
        // Only discard fences at the edges; retain actual fenced examples
        // within a specification.
        if current_path.is_some() && stripped.starts_with("```") {
            if !fenced && content.trim().is_empty() {
                fenced = true;
                continue;
            }
            if fenced {
                fenced = false;
                continue;
            }
        }
        if let Some(path) = stripped
            .strip_prefix("=== FILE: ")
            .and_then(|s| s.strip_suffix(" ==="))
        {
            if current_path.is_some() {
                return Err("Nested FILE record in model response".into());
            }
            if !valid_spec_path(path) {
                return Err(format!(
                    "Invalid specification path in model response: {path}"
                ));
            }
            current_path = Some(path.to_owned());
            content.clear();
            fenced = false;
        } else if stripped == "=== END FILE ===" {
            let path = current_path.take().ok_or("END FILE without FILE")?;
            if content.trim().is_empty() {
                return Err(format!("Empty specification fragment: {path}"));
            }
            records.push((path, content.trim().to_owned()));
            content.clear();
            fenced = false;
        } else if current_path.is_some() {
            // A marker inside a Markdown specification is not a control token.
            // Reject it instead of silently accepting a contradictory record.
            if matches!(stripped, EMPTY_MARKER | "=== NO_MESSAGE_DEFINITIONS ===") {
                return Err("Empty marker inside FILE record".into());
            }
            content.push_str(line);
            content.push('\n');
        } else if matches!(stripped, EMPTY_MARKER | "=== NO_MESSAGE_DEFINITIONS ===") {
            // An extra marker after otherwise complete records is model
            // commentary, not a request to discard the already parsed facts.
            if records.is_empty() {
                return Err("Empty marker combined with other text".into());
            }
        } else if stripped.starts_with("```") || stripped.is_empty() {
            // Tolerate an outer Markdown fence around the record exchange.
        } else if records.is_empty() && !stripped.starts_with("===") {
            // Tolerate an introductory sentence before the first FILE marker.
        } else if !stripped.starts_with("===") {
            // Tolerate a closing explanatory sentence after complete records.
        } else {
            return Err(format!(
                "Unexpected text outside FILE records: {}",
                line.chars().take(160).collect::<String>()
            ));
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

pub fn valid_spec_path(path: &str) -> bool {
    if matches!(path, "general.md" | "infrastructure.md") {
        return true;
    }
    let Some(slug) = path
        .strip_prefix("messages/")
        .and_then(|s| s.strip_suffix(".md"))
    else {
        return false;
    };
    !slug.is_empty()
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// Stage all output files in a fresh sibling directory and publish only after
/// every file has been written. Refuse to overwrite an existing specification.
fn write_spec_directory(
    output: &Path,
    files: &BTreeMap<String, String>,
    source: &Path,
) -> Result<(), String> {
    if output.exists() {
        return Err(format!(
            "Output directory already exists: {}",
            output.display()
        ));
    }
    let name = output
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid output directory name")?;
    let stage = output.with_file_name(format!(".{name}.tmp-{}", std::process::id()));
    fs::create_dir(&stage).map_err(|e| format!("Cannot stage output: {e}"))?;
    let result = (|| {
        fs::create_dir(stage.join("messages")).map_err(|e| e.to_string())?;
        for (path, body) in files {
            fs::write(stage.join(path), format!("{body}\n"))
                .map_err(|e| format!("Cannot write {path}: {e}"))?;
        }
        let mut readme = format!(
            "# Extracted Message Specifications\n\nSource: `{}`\n\n",
            source.display()
        );
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
        let masked_source = mask_opaque_lines(&mask_embedded_data_uris(&source));
        let parts = chunks(&masked_source, CHUNK_CHARS);
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
        let checkpoint_file = checkpoint_path(&output);
        let fingerprint =
            extraction_fingerprint(&source, &[&extract_prompt, &validate_prompt, &merge_prompt]);
        let mut checkpoint = match load_checkpoint(&checkpoint_file, &fingerprint, parts.len()) {
            Ok(Some(checkpoint)) => {
                println!(
                    "## [CHUNKED] RESUME at chunk {}/{}",
                    checkpoint.next_chunk + 1,
                    parts.len()
                );
                checkpoint
            }
            Ok(None) => ExtractionCheckpoint {
                version: CHECKPOINT_VERSION,
                source_fingerprint: fingerprint,
                next_chunk: 0,
                files: BTreeMap::new(),
                source_windows: BTreeMap::new(),
                coverage_next_chunk: 0,
                audit: None,
            },
            Err(err) => return WorkflowResult::Error(err),
        };
        if output.exists() {
            return WorkflowResult::Error(format!(
                "Output directory already exists: {}",
                output.display()
            ));
        }
        let mut files = checkpoint.files.clone();
        for (index, part) in parts.iter().enumerate().skip(checkpoint.next_chunk) {
            if part.text.trim().is_empty() {
                checkpoint.next_chunk = index + 1;
                checkpoint.files = files.clone();
                if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                    return WorkflowResult::Error(err);
                }
                continue;
            }
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
            let records = match parse_file_records(&checked) {
                Ok(records) => records,
                Err(err) => {
                    println!("## [CHUNKED] VALIDATE RETRY {}/{}", index + 1, parts.len());
                    let retried = match self.llm_call.run_text_step_limited_with_temperature(
                        &validate_prompt,
                        &review_context,
                        MAX_TOKENS * 2,
                        0.1,
                    ) {
                        Ok(text) => text,
                        Err(error) => return WorkflowResult::LlmCallResult(error),
                    };
                    match parse_file_records(&retried) {
                        Ok(records) => records,
                        Err(retry_error) => {
                            return WorkflowResult::Error(format!(
                                "Chunk {}: {retry_error} after validation retry; first error: {err}",
                                index + 1
                            ));
                        }
                    }
                }
            };
            for (path, fragment) in records {
                let source_windows = checkpoint.source_windows.entry(path.clone()).or_default();
                if !source_windows.contains(&index) {
                    source_windows.push(index);
                }
                if let Some(existing) = files.get(&path) {
                    println!("## [CHUNKED] MERGE {path}");
                    let merged = append_reviewed_fragment(existing, &fragment);
                    if merged.chars().count() > MAX_MERGE_CHARS {
                        return WorkflowResult::Error(format!(
                            "Cannot safely accumulate {path}: specification exceeds {MAX_MERGE_CHARS} characters"
                        ));
                    }
                    files.insert(path, merged);
                } else {
                    files.insert(path, fragment);
                }
            }
            checkpoint.next_chunk = index + 1;
            checkpoint.files = files.clone();
            if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                return WorkflowResult::Error(err);
            }
        }

        // Independent coverage sweep: every nonempty source window is checked
        // against the assembled file inventory, including windows that emitted
        // NO_MESSAGE_DEFINITIONS during extraction. Persist progress per window.
        // Only genuinely missing identities are admitted; existing specs are
        // never overwritten by the coverage pass.
        let coverage_prompt = format!(
            "{requirements}\n\nReview the source excerpt independently for protocol-defined message identities missing from the provided existing message list. Output only FILE records for genuinely missing messages, using the same record syntax as extraction, or exactly NO_MESSAGE_DEFINITIONS. Do not emit files for identities already listed. Do not infer messages from base64, examples, or isolated fragments."
        );
        for (index, part) in parts
            .iter()
            .enumerate()
            .skip(checkpoint.coverage_next_chunk)
        {
            if !part.text.trim().is_empty() {
                let inventory = files
                    .keys()
                    .filter(|path| path.starts_with("messages/"))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                let context = format!(
                    "SOURCE CHUNK {}/{} LINES {}-{}\n\n=== KNOWN MESSAGES ===\n{inventory}\n\n=== SOURCE ===\n{}",
                    index + 1,
                    parts.len(),
                    part.start_line,
                    part.end_line,
                    part.text
                );
                if context.chars().count() > MAX_MERGE_CHARS {
                    return WorkflowResult::Error(format!(
                        "Coverage context exceeds {MAX_MERGE_CHARS} characters at chunk {}",
                        index + 1
                    ));
                }
                println!("## [CHUNKED] COVERAGE {}/{}", index + 1, parts.len());
                let response = match self.llm_call.run_text_step_limited_with_temperature(
                    &coverage_prompt,
                    &context,
                    MAX_TOKENS,
                    0.0,
                ) {
                    Ok(text) => text,
                    Err(err) => return WorkflowResult::LlmCallResult(err),
                };
                let records = match parse_file_records(&response) {
                    Ok(records) => records,
                    Err(first_error) => {
                        // Coverage is an independent, best-effort discovery pass.
                        // Retry malformed framing once before discarding it.
                        println!("## [CHUNKED] COVERAGE RETRY {}/{}", index + 1, parts.len());
                        let retry_prompt = format!(
                            "{coverage_prompt}\n\nIMPORTANT: Every === FILE: path === record MUST end with a separate === END FILE === line before another FILE record starts. Output no explanatory text. If no new messages exist, output exactly NO_MESSAGE_DEFINITIONS."
                        );
                        let retried = match self.llm_call.run_text_step_limited_with_temperature(
                            &retry_prompt,
                            &context,
                            MAX_TOKENS * 2,
                            0.0,
                        ) {
                            Ok(text) => text,
                            Err(err) => return WorkflowResult::LlmCallResult(err),
                        };
                        match parse_file_records(&retried) {
                            Ok(records) => records,
                            Err(retry_error) => {
                                println!(
                                    "## [CHUNKED] COVERAGE WARNING {}/{}: malformed response after retry ({first_error}; {retry_error}); discovery for this window is incomplete",
                                    index + 1,
                                    parts.len()
                                );
                                Vec::new()
                            }
                        }
                    }
                };
                for (path, fragment) in records {
                    if !coverage_message_path(&path) {
                        // Coverage is discovery-only. Non-message destinations
                        // must never modify general or infrastructure files,
                        // but must not trap resume on a deterministic model error.
                        println!("## [CHUNKED] COVERAGE IGNORE non-message path: {path}");
                        continue;
                    }
                    if files.contains_key(&path) {
                        // A known message is not a coverage discovery.
                        continue;
                    }
                    files.insert(path.clone(), fragment);
                    checkpoint
                        .source_windows
                        .entry(path)
                        .or_default()
                        .push(index);
                }
            }
            checkpoint.coverage_next_chunk = index + 1;
            checkpoint.files = files.clone();
            if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                return WorkflowResult::Error(err);
            }
        }

        if files.is_empty() {
            return WorkflowResult::Error("No protocol specifications were extracted".into());
        }
        files.entry("general.md".into()).or_insert_with(|| {
            "# General Protocol Rules\n\nNo shared rules were established from the source.".into()
        });
        files.entry("infrastructure.md".into()).or_insert_with(|| "# Infrastructure Contracts\n\nNo helper contracts were established from the source.".into());
        // Audit the assembled knowledge, not just individual extraction drafts.
        // Evidence is supplied in bounded windows; changes are reviewed again
        // on subsequent passes. An unchanged pass is a fixed point.
        let audit_prompt = format!(
            "{requirements}\n\n{}",
            WorkflowSteps::ChunkedTransformAudit.get_prompt()
        );
        let decision_prompt = format!(
            "{requirements}\n\n{}",
            WorkflowSteps::ChunkedTransformAuditDecide.get_prompt()
        );
        let audit_fingerprint = extraction_fingerprint("", &[&audit_prompt, &decision_prompt]);
        if checkpoint
            .audit
            .as_ref()
            .is_some_and(|audit| !audit.masked_evidence)
        {
            // Prior decisions were based on raw image payloads; invalidate only
            // audit progress, retaining extraction results and their checkpoint.
            checkpoint.audit = None;
            println!("## [CHUNKED] RESET AUDIT: encoded image evidence is now masked");
        }
        if let Some(audit) = &checkpoint.audit {
            if audit.prompt_fingerprint != audit_fingerprint {
                return WorkflowResult::Error(
                    "Audit prompts changed since checkpoint; cannot safely resume audit".into(),
                );
            }
        } else {
            checkpoint.files = files.clone();
            checkpoint.audit = Some(AuditCheckpoint {
                prompt_fingerprint: audit_fingerprint,
                pass: 0,
                file_index: 0,
                evidence_index: 0,
                changed: false,
                masked_evidence: true,
            });
            if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                return WorkflowResult::Error(err);
            }
        }
        let mut converged = false;
        let mut unresolved_verification = false;
        loop {
            let audit = checkpoint
                .audit
                .as_ref()
                .expect("audit initialized")
                .clone();
            if audit.pass > AUDIT_VERIFICATION_PASS {
                break;
            }
            let paths: Vec<String> = files.keys().cloned().collect();
            if audit.file_index == paths.len() {
                if !audit.changed {
                    converged = true;
                    break;
                }
                let next_pass = audit.pass + 1;
                if next_pass > AUDIT_VERIFICATION_PASS {
                    break;
                }
                checkpoint.audit = Some(AuditCheckpoint {
                    pass: next_pass,
                    file_index: 0,
                    evidence_index: 0,
                    changed: false,
                    ..audit
                });
                checkpoint.files = files.clone();
                if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                    return WorkflowResult::Error(err);
                }
                continue;
            }
            let path = &paths[audit.file_index];
            // Rebuild evidence deterministically from checkpointed files. This
            // also preserves revisions made earlier in the same audit pass.
            let evidence: Vec<(String, String)> = if path == "infrastructure.md" {
                files
                    .iter()
                    .filter(|(name, _)| name.starts_with("messages/") || *name == "general.md")
                    .map(|(name, body)| (name.clone(), body.clone()))
                    .collect()
            } else {
                // Each message is audited only against source windows that
                // actually yielded a validated fragment for that message.
                // Extraction already independently validates every source
                // window, including windows that yield no message.
                let indices: Vec<usize> = if path == "general.md" {
                    // Shared rules can occur alongside any message definition.
                    // Windows that yielded no protocol facts were already
                    // independently validated during extraction.
                    let mut indices: Vec<usize> = checkpoint
                        .source_windows
                        .values()
                        .flat_map(|windows| windows.iter().copied())
                        .collect();
                    indices.sort_unstable();
                    indices.dedup();
                    indices
                } else {
                    checkpoint
                        .source_windows
                        .get(path)
                        .cloned()
                        .unwrap_or_default()
                };
                indices
                    .into_iter()
                    .filter_map(|i| {
                        parts.get(i).map(|part| {
                            (
                                format!(
                                    "source chunk {} lines {}-{}",
                                    i + 1,
                                    part.start_line,
                                    part.end_line
                                ),
                                part.text.clone(),
                            )
                        })
                    })
                    .collect()
            };
            if audit.evidence_index > evidence.len() {
                return WorkflowResult::Error(format!(
                    "Invalid audit checkpoint evidence index for {path}"
                ));
            }
            if audit.evidence_index == evidence.len() {
                let state = checkpoint.audit.as_mut().expect("audit initialized");
                state.file_index += 1;
                state.evidence_index = 0;
                if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                    return WorkflowResult::Error(err);
                }
                continue;
            }
            if evidence[audit.evidence_index].1.trim().is_empty() {
                checkpoint
                    .audit
                    .as_mut()
                    .expect("audit initialized")
                    .evidence_index += 1;
                if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                    return WorkflowResult::Error(err);
                }
                continue;
            }
            let (evidence_name, evidence_body) = &evidence[audit.evidence_index];
            let existing = files.get(path).expect("audit path exists").clone();
            let context = format!(
                "DESTINATION: {path}\nAUDIT PASS: {}\nEVIDENCE: {evidence_name}\n\n=== EXISTING SPECIFICATION ===\n{existing}\n\n=== EVIDENCE ===\n{evidence_body}",
                audit.pass + 1
            );
            if context.chars().count() > MAX_MERGE_CHARS {
                return WorkflowResult::Error(format!(
                    "Audit context exceeds {MAX_MERGE_CHARS} characters for {path} against {evidence_name}"
                ));
            }
            println!(
                "## [CHUNKED] AUDIT CHECK {}/{}: {path} against {evidence_name}",
                audit.pass + 1,
                MAX_AUDIT_PASSES
            );
            let decision = match self.llm_call.run_text_step_limited_with_temperature(
                &decision_prompt,
                &context,
                32,
                0.0,
            ) {
                Ok(text) => text,
                Err(err) => return WorkflowResult::LlmCallResult(err),
            };
            let revise = match parse_audit_decision(&decision) {
                Ok(revise) => revise,
                Err(_) => {
                    // A small model may echo the context or emit a FILE record
                    // instead of the required decision. Retry once with a
                    // minimal instruction; never silently treat this as UNCHANGED.
                    println!(
                        "## [CHUNKED] AUDIT RETRY {}/{}: {path} against {evidence_name}",
                        audit.pass + 1,
                        MAX_AUDIT_PASSES
                    );
                    let retry_prompt = "You are an audit decision classifier. Reply with exactly one token: UNCHANGED or REVISE. Do not echo the input, output Markdown, or produce FILE records. If the evidence is irrelevant or contains only encoded image data, reply UNCHANGED.";
                    let retry_context = format!(
                        "DESTINATION: {path}\nAUDIT PASS: {}\nEVIDENCE: {evidence_name}\n\n=== SPECIFICATION ===\n{existing}\n\n=== SOURCE EVIDENCE ===\n{evidence_body}\n\nOnly answer UNCHANGED or REVISE.",
                        audit.pass + 1
                    );
                    let retried = match self.llm_call.run_text_step_limited_with_temperature(
                        retry_prompt,
                        &retry_context,
                        32,
                        0.0,
                    ) {
                        Ok(text) => text,
                        Err(err) => return WorkflowResult::LlmCallResult(err),
                    };
                    match parse_audit_decision(&retried) {
                        Ok(revise) => revise,
                        Err(_) => {
                            return WorkflowResult::Error(format!(
                                "Invalid audit decision after one retry for {path} against {evidence_name}; first response: {:?}; retry response: {:?}",
                                decision.chars().take(160).collect::<String>(),
                                retried.chars().take(160).collect::<String>()
                            ));
                        }
                    }
                }
            };
            if revise && audit.pass == AUDIT_VERIFICATION_PASS {
                // The final pass is read-only. Preserve the unresolved finding
                // and continue so a complete diagnostic draft can be written.
                println!(
                    "## [CHUNKED] VERIFICATION WARNING: unresolved correction for {path} against {evidence_name}; no change applied"
                );
                unresolved_verification = true;
                checkpoint.files = files.clone();
                checkpoint
                    .audit
                    .as_mut()
                    .expect("audit initialized")
                    .evidence_index += 1;
                if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                    return WorkflowResult::Error(err);
                }
                continue;
            }
            if revise {
                println!(
                    "## [CHUNKED] AUDIT REVISE {}/{}: {path} against {evidence_name}",
                    audit.pass + 1,
                    MAX_AUDIT_PASSES
                );
                let revised = match self.llm_call.run_text_step_limited_with_temperature(
                    &audit_prompt,
                    &context,
                    MAX_TOKENS,
                    0.1,
                ) {
                    Ok(text) => text,
                    Err(err) => return WorkflowResult::LlmCallResult(err),
                };
                let mut accepted_edit = revised.clone();
                let candidate = match apply_audit_edit(&existing, evidence_body, &revised) {
                    Ok(candidate) => candidate,
                    Err(first_err) => {
                        println!(
                            "## [CHUNKED] AUDIT EDIT RETRY {}/{}: {path} against {evidence_name}",
                            audit.pass + 1,
                            MAX_AUDIT_PASSES
                        );
                        let retry_prompt = "Produce ONLY a JSON object with string fields evidence_quote, old_text, new_text. evidence_quote must be a verbatim source substring at least 12 characters long; old_text must be an exact unique substring of the existing specification. Return no Markdown fences or commentary. If no supported change is possible, reply exactly UNCHANGED.";
                        let retry = match self.llm_call.run_text_step_limited_with_temperature(
                            retry_prompt,
                            &context,
                            MAX_TOKENS,
                            0.0,
                        ) {
                            Ok(text) => text,
                            Err(err) => return WorkflowResult::LlmCallResult(err),
                        };
                        if retry.trim() == "UNCHANGED" {
                            println!(
                                "## [CHUNKED] AUDIT SKIP: no supported edit for {path} against {evidence_name}"
                            );
                            existing.clone()
                        } else {
                            match apply_audit_edit(&existing, evidence_body, &retry) {
                                Ok(candidate) => {
                                    accepted_edit = retry;
                                    candidate
                                }
                                Err(retry_err) => {
                                    // Never apply an edit that cannot be located
                                    // unambiguously. Record the unresolved audit
                                    // finding and continue checking other evidence
                                    // instead of retrying the same checkpoint forever.
                                    println!(
                                        "## [CHUNKED] AUDIT WARNING: unresolved edit for {path} against {evidence_name}; first: {first_err}; retry: {retry_err}; no change applied"
                                    );
                                    existing.clone()
                                }
                            }
                        }
                    }
                };
                if candidate == existing {
                    checkpoint.files = files.clone();
                    checkpoint
                        .audit
                        .as_mut()
                        .expect("audit initialized")
                        .evidence_index += 1;
                    if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                        return WorkflowResult::Error(err);
                    }
                    continue;
                }
                let verify_prompt = "Independently verify the proposed minimal edit. The caller has already checked that old_text occurs exactly once in the destination and that the source quote is verbatim. Reply exactly APPROVE only if the source evidence supports replacing old_text with new_text without introducing unsupported claims. Otherwise reply exactly REJECT.";
                // The edit was already validated by apply_audit_edit.
                // Use its exact evidence quote, not the entire source window:
                // some windows contain very large embedded data or long edits.
                let normalized_edit = strip_code_fences(accepted_edit.trim());
                let edit: AuditEdit = match serde_json::from_str(&normalized_edit) {
                    Ok(edit) => edit,
                    Err(err) => {
                        return WorkflowResult::Error(format!(
                            "Previously validated audit edit could not be parsed for {path}: {err}"
                        ));
                    }
                };
                let verify_context = format!(
                    "DESTINATION: {path}\nEVIDENCE: {evidence_name}\n\n=== VERBATIM SOURCE QUOTE ===\n{}\n\n=== OLD TEXT ===\n{}\n\n=== NEW TEXT ===\n{}",
                    edit.evidence_quote, edit.old_text, edit.new_text
                );
                if verify_context.chars().count() > MAX_MERGE_CHARS {
                    // A valid but oversized edit is not safe to send to the
                    // verifier. Keep the original and advance with a warning.
                    println!(
                        "## [CHUNKED] AUDIT WARNING: verification edit exceeds {MAX_MERGE_CHARS} characters for {path} against {evidence_name}; no change applied"
                    );
                    checkpoint.files = files.clone();
                    checkpoint
                        .audit
                        .as_mut()
                        .expect("audit initialized")
                        .evidence_index += 1;
                    if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                        return WorkflowResult::Error(err);
                    }
                    continue;
                }
                let verdict = match self.llm_call.run_text_step_limited_with_temperature(
                    verify_prompt,
                    &verify_context,
                    32,
                    0.0,
                ) {
                    Ok(text) => text,
                    Err(err) => return WorkflowResult::LlmCallResult(err),
                };
                if verdict.trim() != "APPROVE" {
                    // A rejected proposal must never change the specification.
                    // Record the unresolved finding and checkpoint this evidence
                    // so a deterministic REJECT cannot trap resume indefinitely.
                    println!(
                        "## [CHUNKED] AUDIT WARNING: edit not independently approved for {path} against {evidence_name}: {:?}; no change applied",
                        verdict.chars().take(160).collect::<String>()
                    );
                    checkpoint.files = files.clone();
                    checkpoint
                        .audit
                        .as_mut()
                        .expect("audit initialized")
                        .evidence_index += 1;
                    if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                        return WorkflowResult::Error(err);
                    }
                    continue;
                }
                files.insert(path.clone(), candidate);
                checkpoint
                    .audit
                    .as_mut()
                    .expect("audit initialized")
                    .changed = true;
            }
            // Advance only after a complete decision (and any revision), with
            // the resulting files and cursor saved together atomically.
            checkpoint.files = files.clone();
            checkpoint
                .audit
                .as_mut()
                .expect("audit initialized")
                .evidence_index += 1;
            if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                return WorkflowResult::Error(err);
            }
        }
        if !converged {
            return WorkflowResult::Error(format!(
                "Specification audit did not converge after {MAX_AUDIT_PASSES} revision passes and one final verification pass; refusing unverified output"
            ));
        }
        // Never publish an unverified draft at the normal destination.
        // Keep the checkpoint so the unresolved findings can be revisited.
        let destination = if unresolved_verification {
            output.with_file_name(format!(
                "{}-unverified",
                output.file_name().unwrap_or_default().to_string_lossy()
            ))
        } else {
            output.clone()
        };
        match write_spec_directory(&destination, &files, &input) {
            Ok(()) => {
                if unresolved_verification {
                    println!(
                        "## [CHUNKED] WROTE UNVERIFIED DRAFT {}; unresolved verification findings remain; checkpoint retained",
                        destination.display()
                    );
                } else {
                    println!("## [CHUNKED] WROTE {}", destination.display());
                    if let Err(err) = fs::remove_file(&checkpoint_file) {
                        return WorkflowResult::Error(format!(
                            "Output published, but cannot remove checkpoint: {err}"
                        ));
                    }
                }
                WorkflowResult::Ok
            }
            Err(err) => WorkflowResult::Error(err),
        }
    }
}
