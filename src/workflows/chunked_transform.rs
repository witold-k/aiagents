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

fn legacy_unresolved_findings() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
struct ExtractionCheckpoint {
    version: u32,
    source_fingerprint: String,
    next_chunk: usize,
    files: BTreeMap<String, String>,
    #[serde(default)]
    facts: BTreeMap<String, Vec<SourceFact>>,
    #[serde(default)]
    source_windows: BTreeMap<String, Vec<usize>>,
    #[serde(default)]
    coverage_next_chunk: usize,
    // Legacy v5 checkpoints did not track warnings. Treat them as unverified.
    #[serde(default = "legacy_unresolved_findings")]
    unresolved_findings: bool,
    #[serde(default)]
    audit: Option<AuditCheckpoint>,
    #[serde(default)]
    fact_audit_cursor: usize,
    #[serde(default)]
    fact_audit_version: u32,
}

/// One source-attributed, independently deduplicated specification assertion.
/// A source window is evidence location, not a claim of verbatim verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SourceFact {
    statement: String,
    source_chunk: usize,
    start_line: usize,
    end_line: usize,
    #[serde(default)]
    evidence_quote: Option<String>,
    #[serde(default)]
    review: FactReview,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum FactReview {
    #[default]
    Pending,
    Supported,
    Unresolved,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FactEvidence {
    evidence_quote: String,
}

fn verified_fact_quote(response: &str, source: &str) -> Result<String, String> {
    let normalized = strip_code_fences(response.trim());
    let proof: FactEvidence = serde_json::from_str(&normalized)
        .map_err(|err| format!("Invalid fact evidence JSON: {err}"))?;
    let quote = proof.evidence_quote.trim();
    if quote.chars().count() < 12 || !source.contains(quote) {
        return Err("Fact quote is too short or absent from the attributed source window".into());
    }
    Ok(quote.to_owned())
}

fn record_source_facts(
    facts: &mut BTreeMap<String, Vec<SourceFact>>,
    path: &str,
    fragment: &str,
    chunk: usize,
    start_line: usize,
    end_line: usize,
) {
    let entries = facts.entry(path.to_owned()).or_default();
    for statement in markdown_blocks(fragment) {
        if !entries.iter().any(|fact| fact.statement == statement) {
            entries.push(SourceFact {
                statement,
                source_chunk: chunk,
                start_line,
                end_line,
                evidence_quote: None,
                review: FactReview::Pending,
            });
        }
    }
}

fn render_source_facts(facts: &[SourceFact]) -> String {
    facts
        .iter()
        .map(|fact| fact.statement.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Derive a canonical identity only from an explicit command/message token.
/// An arbitrary descriptive filename is never treated as an identity.
fn canonical_message_path(path: &str, fragment: &str) -> String {
    if !path.starts_with("messages/") {
        return path.to_owned();
    }
    for line in fragment.lines().take(12) {
        let heading = line.trim().trim_start_matches('#').trim();
        let lower = heading.to_ascii_lowercase();
        for prefix in ["message ", "command ", "message: ", "command: "] {
            if let Some(rest) = lower.strip_prefix(prefix) {
                let token = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c: char| !c.is_ascii_alphanumeric());
                if token.len() == 1 && token.bytes().all(|b| b.is_ascii_alphanumeric()) {
                    return format!("messages/{token}.md");
                }
            }
        }
    }
    path.to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditCheckpoint {
    // Only the legacy cursor bounds are needed while reading v5 checkpoints.
    pass: usize,
    file_index: usize,
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

/// Split Markdown on blank lines outside fenced examples. A table or fenced
/// example stays intact; no semantic interpretation of protocol data occurs.
fn markdown_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~');
        if let Some(marker) = marker {
            let count = trimmed.chars().take_while(|c| *c == marker).count();
            if count >= 3 {
                match fence {
                    None => fence = Some((marker, count)),
                    Some((open, length)) if open == marker && count >= length => fence = None,
                    _ => {}
                }
            }
        }
        if fence.is_none() && line.trim().is_empty() {
            if !current.trim().is_empty() {
                blocks.push(current.trim().to_owned());
                current.clear();
            }
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }
    if !current.trim().is_empty() {
        blocks.push(current.trim().to_owned());
    }
    blocks
}

/// Deterministically consolidate identical reviewed Markdown blocks. Different
/// statements, including contradictions, remain available for the audit.
pub fn append_reviewed_fragment(existing: &str, fragment: &str) -> String {
    let new_fragment = fragment.trim();
    if new_fragment.is_empty() || existing.trim() == new_fragment {
        return existing.to_owned();
    }
    let existing_blocks = markdown_blocks(existing);
    let mut seen: std::collections::BTreeSet<String> = existing_blocks.into_iter().collect();
    let mut additions = Vec::new();
    for block in markdown_blocks(new_fragment) {
        if !seen.insert(block.clone()) {
            continue;
        }
        additions.push(block);
    }
    if additions.is_empty() {
        existing.to_owned()
    } else {
        format!("{existing}\n\n{}", additions.join("\n\n"))
    }
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

/// Internal workflow envelopes are never protocol facts.
fn internal_workflow_marker(line: &str) -> bool {
    matches!(
        line.trim(),
        "=== CONTEXT ==="
            | "=== EXISTING SPECIFICATION ==="
            | "=== NEW REVIEWED FRAGMENT ==="
            | "=== CONSOLIDATED SPECIFICATION ==="
            | "=== SOURCE ==="
            | "=== EVIDENCE ==="
            | "=== PROPOSED EDIT ==="
    )
}

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
            // Internal LLM prompt/merge envelopes must never become published
            // protocol content. They indicate a contaminated model response.
            if internal_workflow_marker(stripped) {
                return Err(format!(
                    "Model leaked internal workflow marker into specification: {stripped}"
                ));
            }
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
    unverified: bool,
    source_windows: &BTreeMap<String, Vec<usize>>,
    facts: &BTreeMap<String, Vec<SourceFact>>,
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
        if unverified {
            readme.push_str("\n## Verification status\n\n**UNVERIFIED DIAGNOSTIC DRAFT.** Coverage or audit findings remain unresolved. Do not use these specifications as an authoritative basis for implementation.\n");
        } else {
            readme.push_str("\n## Verification status\n\nNo unresolved findings were recorded by the workflow. Model review is not a proof of protocol correctness; independently verify against the source before implementation.\n");
        }
        readme.push_str("\n## Extraction provenance\n\nSource-window indices are 1-based and refer to the overlapping source windows used during extraction. They are not a substitute for exact source citations.\n\n");
        for (path, indices) in source_windows {
            let mut indices = indices.clone();
            indices.sort_unstable();
            indices.dedup();
            let windows = indices
                .iter()
                .map(|i| (i + 1).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            readme.push_str(&format!("- `{path}`: windows {windows}\n"));
        }
        readme.push_str("\n## Structured facts\n\nSee [facts.json](facts.json) for deduplicated assertions, source-window attribution, evidence_quote and review status (pending/supported/unresolved). Supported means a verbatim quote exists in the attributed window, not that the assertion is semantically proven. Historical checkpoint attribution may be unreliable.\n");
        let fact_json = serde_json::to_vec_pretty(facts)
            .map_err(|e| format!("Cannot serialize structured facts: {e}"))?;
        fs::write(stage.join("facts.json"), fact_json)
            .map_err(|e| format!("Cannot write facts.json: {e}"))?;
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
                facts: BTreeMap::new(),
                source_windows: BTreeMap::new(),
                coverage_next_chunk: 0,
                unresolved_findings: false,
                audit: None,
                fact_audit_cursor: 0,
                fact_audit_version: 0,
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
            for (model_path, fragment) in records {
                let path = canonical_message_path(&model_path, &fragment);
                if fragment.chars().count() > MAX_MERGE_CHARS {
                    return WorkflowResult::Error(format!(
                        "Extracted fragment for {path} exceeds {MAX_MERGE_CHARS} characters"
                    ));
                }
                let source_windows = checkpoint.source_windows.entry(path.clone()).or_default();
                if !source_windows.contains(&index) {
                    source_windows.push(index);
                }
                // Facts are the canonical accumulation format. Large shared
                // specifications no longer deadlock at a fixed Markdown size.
                // Preserve legacy checkpoint content as facts on first update.
                if !checkpoint.facts.contains_key(&path)
                    && let Some(existing) = files.get(&path)
                {
                    record_source_facts(&mut checkpoint.facts, &path, existing, usize::MAX, 0, 0);
                    checkpoint.unresolved_findings = true;
                }
                record_source_facts(
                    &mut checkpoint.facts,
                    &path,
                    &fragment,
                    index,
                    part.start_line,
                    part.end_line,
                );
                if files.contains_key(&path) {
                    println!("## [CHUNKED] MERGE {path}");
                }
                files.insert(path.clone(), render_source_facts(&checkpoint.facts[&path]));
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
                                checkpoint.unresolved_findings = true;
                                Vec::new()
                            }
                        }
                    }
                };
                for (model_path, fragment) in records {
                    let path = canonical_message_path(&model_path, &fragment);
                    if !coverage_message_path(&path) {
                        // Coverage is discovery-only. Non-message destinations
                        // must never modify general or infrastructure files,
                        // but must not trap resume on a deterministic model error.
                        println!("## [CHUNKED] COVERAGE IGNORE non-message path: {path}");
                        checkpoint.unresolved_findings = true;
                        continue;
                    }
                    if files.contains_key(&path) {
                        // A known message is not a coverage discovery.
                        continue;
                    }
                    if fragment.chars().count() > MAX_MERGE_CHARS {
                        return WorkflowResult::Error(format!(
                            "Coverage fragment for {path} exceeds {MAX_MERGE_CHARS} characters"
                        ));
                    }
                    record_source_facts(
                        &mut checkpoint.facts,
                        &path,
                        &fragment,
                        index,
                        part.start_line,
                        part.end_line,
                    );
                    files.insert(path.clone(), render_source_facts(&checkpoint.facts[&path]));
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
        // Review individual immutable facts against their attributed source windows.
        // Fact audit v1 intentionally invalidates the old file-edit audit cursor.
        // The extraction, coverage and source facts remain checkpointed.
        if checkpoint.fact_audit_version != 1 {
            checkpoint.fact_audit_version = 1;
            checkpoint.fact_audit_cursor = 0;
            if checkpoint.audit.is_some() {
                checkpoint.unresolved_findings = true;
            }
            checkpoint.audit = None;
            println!("## [CHUNKED] RESET AUDIT: switching to source-grounded fact review");
            if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                return WorkflowResult::Error(err);
            }
        }
        // Import legacy file bodies without inventing source provenance.
        for (path, body) in &files {
            if !checkpoint.facts.contains_key(path) {
                record_source_facts(&mut checkpoint.facts, path, body, usize::MAX, 0, 0);
                checkpoint.unresolved_findings = true;
            }
        }
        let fact_keys: Vec<(String, usize)> = checkpoint
            .facts
            .iter()
            .flat_map(|(path, facts)| (0..facts.len()).map(move |index| (path.clone(), index)))
            .collect();
        if checkpoint.fact_audit_cursor > fact_keys.len() {
            return WorkflowResult::Error("Invalid fact audit checkpoint cursor".into());
        }
        while checkpoint.fact_audit_cursor < fact_keys.len() {
            let (path, index) = &fact_keys[checkpoint.fact_audit_cursor];
            let fact = checkpoint.facts[path][*index].clone();
            // Older v5 migrations assigned an entire accumulated file to the
            // extraction window that happened to trigger migration. If many
            // distinct facts share one such window, attribution is ambiguous.
            // Do not spend one LLM call per fact on that unreliable premise.
            let suspicious_legacy_batch = checkpoint.facts[path]
                .iter()
                .filter(|other| other.source_chunk == fact.source_chunk)
                .count()
                > 20;
            let source = if suspicious_legacy_batch {
                None
            } else {
                parts.get(fact.source_chunk).map(|part| part.text.as_str())
            };
            // A historical checkpoint can contain entire Markdown documents
            // assigned to a single window. That is NOT trustworthy provenance.
            let result = if let Some(source) = source {
                if fact.statement.chars().count() > 4_000 || source.trim().is_empty() {
                    Err("Oversized fact or empty source window".to_owned())
                } else {
                    println!(
                        "## [CHUNKED] FACT AUDIT {}/{}: {path} fact {}",
                        checkpoint.fact_audit_cursor + 1,
                        fact_keys.len(),
                        index + 1
                    );
                    let context = format!(
                        "DESTINATION: {path}\\nSOURCE WINDOW: {} (lines {}-{})\\n\\n=== FACT ===\\n{}\\n\\n=== ORIGINAL SOURCE ===\\n{}\\n",
                        fact.source_chunk + 1,
                        fact.start_line,
                        fact.end_line,
                        fact.statement,
                        source
                    );
                    let prompt = r#"Find a verbatim quotation in ORIGINAL SOURCE that directly supports the FACT. Return ONLY JSON: {"evidence_quote":"exact source substring at least 12 characters"}. If there is no supporting quotation, return exactly UNSUPPORTED. Do not invent quotes or rewrite facts."#;
                    match self
                        .llm_call
                        .run_text_step_limited_with_temperature(prompt, &context, 512, 0.0)
                    {
                        Ok(answer) if answer.trim() == "UNSUPPORTED" => {
                            Err("Model did not find supporting source evidence".to_owned())
                        }
                        Ok(answer) => verified_fact_quote(&answer, source),
                        Err(err) => return WorkflowResult::LlmCallResult(err),
                    }
                }
            } else {
                Err("Missing, ambiguous legacy, or unattributed original source window".to_owned())
            };
            let current = &mut checkpoint.facts.get_mut(path).expect("fact path exists")[*index];
            match result {
                Ok(quote) => {
                    current.evidence_quote = Some(quote);
                    current.review = FactReview::Supported;
                }
                Err(reason) => {
                    println!(
                        "## [CHUNKED] FACT WARNING: {path} fact {}: {reason}",
                        index + 1
                    );
                    current.evidence_quote = None;
                    current.review = FactReview::Unresolved;
                    checkpoint.unresolved_findings = true;
                }
            }
            checkpoint.fact_audit_cursor += 1;
            // Persist review outcome and cursor in the same atomic checkpoint.
            if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                return WorkflowResult::Error(err);
            }
        }
        // Markdown is a projection of immutable extracted assertions. The
        // audit never rewrites it independently of the facts in facts.json.
        for (path, facts) in &checkpoint.facts {
            files.insert(path.clone(), render_source_facts(facts));
        }
        checkpoint.files = files.clone();
        if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
            return WorkflowResult::Error(err);
        }
        // Older checkpoints may already contain prompt envelopes from
        // responses accepted before the parser was hardened.
        for (path, body) in &files {
            if let Some(marker) = body.lines().find(|line| internal_workflow_marker(line)) {
                println!(
                    "## [CHUNKED] OUTPUT WARNING: internal workflow marker in {path}: {}; diagnostic draft only",
                    marker.trim()
                );
                checkpoint.unresolved_findings = true;
            }
        }
        // Never publish an unverified draft at the normal destination.
        // Keep the checkpoint so the unresolved findings can be revisited.
        let destination = if checkpoint.unresolved_findings {
            output.with_file_name(format!(
                "{}-unverified",
                output.file_name().unwrap_or_default().to_string_lossy()
            ))
        } else {
            output.clone()
        };
        match write_spec_directory(
            &destination,
            &files,
            &input,
            checkpoint.unresolved_findings,
            &checkpoint.source_windows,
            &checkpoint.facts,
        ) {
            Ok(()) => {
                if checkpoint.unresolved_findings {
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

#[cfg(test)]
mod checkpoint_review_tests {
    use super::*;

    #[test]
    fn legacy_checkpoint_without_warning_field_is_unverified() {
        let checkpoint: ExtractionCheckpoint = serde_json::from_str(
            r#"{"version":5,"source_fingerprint":"test","next_chunk":0,"files":{},"source_windows":{},"coverage_next_chunk":0,"audit":null}"#,
        )
        .unwrap();
        assert!(checkpoint.unresolved_findings);
    }

    #[test]
    fn explicitly_clean_checkpoint_stays_clean() {
        let checkpoint: ExtractionCheckpoint = serde_json::from_str(
            r#"{"version":5,"source_fingerprint":"test","next_chunk":0,"files":{},"source_windows":{},"coverage_next_chunk":0,"unresolved_findings":false,"audit":null}"#,
        )
        .unwrap();
        assert!(!checkpoint.unresolved_findings);
    }

    #[test]
    fn detects_legacy_internal_envelopes() {
        assert!(internal_workflow_marker(
            "=== CONSOLIDATED SPECIFICATION ==="
        ));
        assert!(!internal_workflow_marker("# General protocol rules"));
    }
}

#[cfg(test)]
mod structured_fact_tests {
    use super::*;

    #[test]
    fn repeated_facts_are_deduplicated_with_first_source_retained() {
        let mut facts = BTreeMap::new();
        record_source_facts(&mut facts, "general.md", "Rule A.\n\nRule B.", 2, 10, 15);
        record_source_facts(&mut facts, "general.md", "Rule A.\n\nRule C.", 3, 14, 20);
        let entries = &facts["general.md"];
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].source_chunk, 2);
        assert_eq!(entries[0].start_line, 10);
        assert_eq!(
            render_source_facts(entries),
            "Rule A.\n\nRule B.\n\nRule C."
        );
    }

    #[test]
    fn canonical_identity_requires_explicit_command_heading() {
        assert_eq!(
            canonical_message_path("messages/keepalive.md", "# Command A\n\nDescription"),
            "messages/a.md"
        );
        assert_eq!(
            canonical_message_path("messages/beat_frequency.md", "# Message B\n\nDescription"),
            "messages/b.md"
        );
        assert_eq!(
            canonical_message_path("messages/keepalive.md", "# Keepalive\n\nDescription"),
            "messages/keepalive.md"
        );
        assert_eq!(
            canonical_message_path("general.md", "# Command A"),
            "general.md"
        );
    }

    #[test]
    fn fact_evidence_requires_verbatim_source_quote() {
        let source = "The timeout is exactly 30 seconds in this mode.";
        assert_eq!(
            verified_fact_quote(
                r#"{"evidence_quote":"timeout is exactly 30 seconds"}"#,
                source,
            )
            .unwrap(),
            "timeout is exactly 30 seconds"
        );
        assert!(
            verified_fact_quote(
                r#"{"evidence_quote":"timeout is exactly 60 seconds"}"#,
                source,
            )
            .is_err()
        );
        assert!(verified_fact_quote(r#"{"evidence_quote":"timeout"}"#, source).is_err());
        assert!(verified_fact_quote("UNSUPPORTED", source).is_err());
    }

    #[test]
    fn legacy_fact_review_defaults_to_pending() {
        let fact: SourceFact = serde_json::from_str(
            r#"{"statement":"A","source_chunk":1,"start_line":2,"end_line":3}"#,
        )
        .unwrap();
        assert_eq!(fact.review, FactReview::Pending);
        assert_eq!(fact.evidence_quote, None);
    }

    #[test]
    fn contradictory_facts_are_preserved_for_review() {
        let mut facts = BTreeMap::new();
        record_source_facts(&mut facts, "general.md", "Timeout is zero.", 0, 1, 3);
        record_source_facts(&mut facts, "general.md", "Timeout is nonzero.", 1, 4, 6);
        assert_eq!(facts["general.md"].len(), 2);
    }
}
