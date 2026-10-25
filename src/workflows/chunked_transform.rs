// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Bounded, source-aware document transformation with independent review.

// P4 staging preserves source-attributed candidates before identity resolution.
#[path = "message_identity.rs"]
mod message_identity;

use serde::{Deserialize, Serialize};

use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::{Path, PathBuf},
};

use crate::generated_tasks::Tasks;
use crate::runtimetools::llmcall::LlmCall;
use crate::utils::stringutils::{normalize_html_tables, resolve_source_quote, strip_code_fences};
// Preserve the existing workflow API while the string processing lives in stringutils.
pub use crate::utils::stringutils::{mask_embedded_data_uris, mask_opaque_lines};
use crate::workflows::workflow::{Workflow, WorkflowResult};

const CHUNK_CHARS: usize = 4_000;
pub const CHUNK_OVERLAP_CHARS: usize = 1_000;
const MAX_TOKENS: u32 = 2_048;
const MIN_RECOVERY_CHARS: usize = 700;
const MAX_RECOVERY_WINDOWS: usize = 32;
const MAX_MERGE_CHARS: usize = 20_000;
const MAX_AUDIT_PASSES: usize = 2;
const AUDIT_VERIFICATION_PASS: usize = MAX_AUDIT_PASSES;

const CHECKPOINT_VERSION: u32 = 16;

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
    unresolved_findings: bool,
    #[serde(default)]
    audit: Option<AuditCheckpoint>,
    #[serde(default)]
    fact_audit_cursor: usize,
    #[serde(default)]
    fact_audit_version: u32,
    /// Version of the deterministic candidate-to-message projection, separate from review.
    #[serde(default)]
    identity_projection_version: u32,
    #[serde(default)]
    recovery_chunk: Option<usize>,
    #[serde(default)]
    recovery_pending: Vec<String>,
    #[serde(default)]
    recovery_records: ExtractedRecords,
    #[serde(default)]
    candidates: Vec<message_identity::MessageCandidate>,
    #[serde(default)]
    recovery_candidates: Vec<message_identity::MessageCandidate>,
}

// v7 stored the projection generation in the fact-review version field.
// Recognize that already-completed projection without replaying LLM work.
fn migrate_identity_projection_version(checkpoint: &mut ExtractionCheckpoint) {
    if checkpoint.identity_projection_version == 0 && checkpoint.fact_audit_version == 7 {
        checkpoint.identity_projection_version = 1;
    }
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
    /// Exact byte offset within source_chunk, if available from P4 staging.
    #[serde(default)]
    source_byte_offset: Option<usize>,
    /// All distinct source occurrences retained when identical statements merge.
    #[serde(default)]
    evidence_occurrences: Vec<FactEvidenceOccurrence>,
    #[serde(default)]
    review: FactReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FactEvidenceOccurrence {
    source_chunk: usize,
    start_line: usize,
    end_line: usize,
    evidence_quote: Option<String>,
    #[serde(default)]
    source_byte_offset: Option<usize>,
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
    resolve_source_quote(source, &proof.evidence_quote)
        .ok_or_else(|| "Fact quote is too short or absent from the attributed source window".into())
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedFact {
    statement: String,
    #[serde(default)]
    source_anchor: Option<String>,
    #[serde(default)]
    evidence_quote: Option<String>,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedFile {
    path: String,
    facts: Vec<ExtractedFact>,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedWindow {
    files: Vec<ExtractedFile>,
}

type ExtractedRecords = Vec<(String, Vec<(String, Option<String>)>)>;

#[derive(Debug)]
struct EvidenceSegment {
    id: String,
    text: String,
    line_index: usize,
    /// Byte offset within the source window passed to evidence_segments.
    byte_offset: usize,
}

fn evidence_segments(source: &str) -> Vec<EvidenceSegment> {
    let mut segments = Vec::new();
    let mut line_start = 0;
    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        let mut rest = line;
        let mut consumed = 0;
        while !rest.is_empty() {
            let offsets: Vec<usize> = rest
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(rest.len()))
                .collect();
            let limit = 320.min(offsets.len() - 1);
            let mut end = offsets[limit];
            if limit < offsets.len() - 1 {
                // Prefer a complete table cell; fall back to a UTF-8-safe
                // character boundary only for an oversized individual cell.
                if let Some(cell_end) = rest[..end].rfind(" | ")
                    && cell_end > 0
                {
                    end = cell_end + 3;
                }
            }
            let text = &rest[..end];
            if !text.trim().is_empty() {
                segments.push(EvidenceSegment {
                    id: format!("S{}", segments.len() + 1),
                    text: text.to_owned(),
                    line_index,
                    byte_offset: line_start + consumed,
                });
            }
            rest = &rest[end..];
            consumed += end;
        }
        line_start += line.len();
    }
    segments
}

// Context is read from the source before the current window, never from a
// previous model response. This makes resume and recovery splits deterministic.
// It is a hint to the model, not evidence for a fact or a message identity.
fn preceding_extraction_context(source: &str, part: &Chunk, window: &str) -> String {
    let local_offset = part.text.find(window).unwrap_or(0);
    let before = &source[..(part.start_byte + local_offset).min(source.len())];
    let tail_start = before
        .char_indices()
        .rev()
        .nth(1_199)
        .map_or(0, |(offset, _)| offset);
    let tail = &before[tail_start..];
    format!(
        "=== PRECEDING SOURCE CONTEXT (read-only; not evidence) ===\n{tail}\n=== END PRECEDING CONTEXT ===\n         Use this context only to understand the current section or message type.          A type remains applicable only while the source structure supports it.          A new heading, message declaration, table boundary or unrelated section may end it.          Preserve uppercase/lowercase exactly (Type: G differs from Type: g).          Every emitted fact must cite an S-number from CURRENT SOURCE below;          never cite the preceding context as evidence.\n\n"
    )
}

fn segmented_source(source: &str) -> String {
    evidence_segments(source)
        .iter()
        .map(|segment| format!("[{}] {}\n", segment.id, segment.text.trim_end_matches('\n')))
        .collect()
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct SegmentedFact {
    statement: String,
    // Legacy model responses may use source_anchor for the locator.
    // Only an actual local S-number resolves to evidence; arbitrary
    // copied text never becomes an authoritative quote.
    #[serde(alias = "source_anchor")]
    segment_id: String,
    // Some models repeat the file identity inside each fact. Accept this
    // specific misplaced field, but validate it against the source below.
    #[serde(default)]
    identity_segment_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SegmentedFile {
    path: String,
    #[serde(default)]
    identity_segment_id: Option<String>,
    facts: Vec<SegmentedFact>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SegmentedWindow {
    files: Vec<SegmentedFile>,
}

// Preserve the case of one-letter protocol types. OpenAMIP uses distinct
// uppercase modem and lowercase antenna messages (e.g. X and x).
// Legacy messages/x.md is accepted for uppercase X only when no lowercase
// x declaration exists. In the ambiguous case, uppercase uses upper_x.md.
fn declared_message_identity(path: &str, source: &str) -> bool {
    let slug = path
        .strip_prefix("messages/")
        .and_then(|s| s.strip_suffix(".md"))
        .unwrap_or("");
    let (wanted, explicit_upper) = if let Some(letter) = slug.strip_prefix("upper_") {
        (letter, true)
    } else {
        (slug, false)
    };
    if wanted.len() != 1 || !wanted.bytes().all(|b| b.is_ascii_lowercase()) {
        return false;
    }
    let letter = wanted.as_bytes()[0];
    let mut has_upper = false;
    let mut has_lower = false;
    for line in source.lines() {
        let text = line.trim().trim_start_matches('#').trim();
        let declared = [
            "Type:", "Command:", "Message:", "Command ", "Message ", "Cmd ",
        ]
        .iter()
        .find_map(|prefix| {
            text.get(..prefix.len())
                .filter(|head| head.eq_ignore_ascii_case(prefix))
                .map(|_| text[prefix.len()..].trim_start())
        })
        .or_else(|| {
            // Markdown tables emitted by PDF-to-text tools also declare
            // types. Only a single-letter first cell is authoritative.
            let rest = text.strip_prefix('|')?.trim_start();
            Some(rest)
        });
        if let Some(value) = declared {
            let token = value
                .trim_start_matches(|c: char| c.is_whitespace() || c == '|')
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .next()
                .unwrap_or("");
            if token.len() == 1 && token.as_bytes()[0].to_ascii_lowercase() == letter {
                has_upper |= token.as_bytes()[0].is_ascii_uppercase();
                has_lower |= token.as_bytes()[0].is_ascii_lowercase();
            }
        }
    }
    if explicit_upper {
        has_upper
    } else {
        has_lower || (has_upper && !has_lower)
    }
}

#[cfg(test)]
fn parse_segmented_window(response: &str, source: &str) -> Result<ExtractedRecords, String> {
    parse_segmented_window_with_identity(response, source, source)
}

// An explicit type label inside an assertion is stronger evidence of ownership
// than the destination chosen by the model. Reject contradictory labels instead
// of allowing a later quote audit to mark the wrong message as supported.
fn explicit_type_label(statement: &str) -> Option<u8> {
    let text = statement.trim_start().trim_start_matches('|').trim_start();
    let rest = text.strip_prefix("Type:")?.trim_start();
    let letter = rest.as_bytes().first().copied()?;
    if !letter.is_ascii_alphabetic() {
        return None;
    }
    if rest
        .as_bytes()
        .get(1)
        .is_some_and(|next| next.is_ascii_alphanumeric() || *next == b'_')
    {
        return None;
    }
    Some(letter)
}

fn message_path_type(path: &str, identity_source: &str) -> Option<u8> {
    let slug = path.strip_prefix("messages/")?.strip_suffix(".md")?;
    if let Some(upper) = slug.strip_prefix("upper_") {
        return (upper.len() == 1 && upper.as_bytes()[0].is_ascii_lowercase())
            .then(|| upper.as_bytes()[0].to_ascii_uppercase());
    }
    if slug.len() != 1 || !slug.as_bytes()[0].is_ascii_lowercase() {
        return None;
    }
    let lower = slug.as_bytes()[0];
    let upper_path = format!("messages/upper_{slug}.md");
    let has_upper = declared_message_identity(&upper_path, identity_source);
    let has_lower = identity_source
        .lines()
        .any(|line| explicit_type_label(line.trim().trim_start_matches('#').trim()) == Some(lower));
    if has_upper && has_lower {
        None
    } else {
        Some(if has_upper {
            lower.to_ascii_uppercase()
        } else {
            lower
        })
    }
}

// When both cases are declared, legacy messages/x.md is ambiguous. Only
// an explicit statement or source identity may resolve it.
fn fact_message_path(
    path: &str,
    fact_type: Option<u8>,
    identity_source: &str,
) -> Result<String, String> {
    let Some(slug) = path
        .strip_prefix("messages/")
        .and_then(|p| p.strip_suffix(".md"))
    else {
        return Ok(path.to_owned());
    };
    if slug.len() != 1 || !slug.as_bytes()[0].is_ascii_lowercase() {
        return Ok(path.to_owned());
    }
    let lower = slug.as_bytes()[0];
    let upper_path = format!("messages/upper_{slug}.md");
    let has_upper = declared_message_identity(&upper_path, identity_source);
    let has_lower = identity_source
        .lines()
        .any(|line| explicit_type_label(line.trim().trim_start_matches('#').trim()) == Some(lower));
    if has_upper && has_lower {
        return match fact_type {
            Some(letter) if letter == lower => Ok(path.to_owned()),
            Some(letter) if letter == lower.to_ascii_uppercase() => Ok(upper_path),
            Some(letter) => Err(format!(
                "Fact declares Type: {} but destination {} is a different message",
                letter as char, path
            )),
            None => Err(format!(
                "Ambiguous uppercase/lowercase identity for {path}: fact has no explicit type evidence"
            )),
        };
    }
    Ok(path.to_owned())
}

// Only an isolated Type: heading owns subsequent lines. A table row or a
// descriptive Type: A | ... entry does not establish an open-ended section.
// Blank lines and new headings end ownership; unrelated source material must
// remain unclassified rather than inherit a distant message type.
fn section_type_for_line(source: &str, line_index: usize) -> Option<u8> {
    let lines: Vec<&str> = source.lines().collect();
    if line_index >= lines.len() {
        return None;
    }
    for distance in 0..=12 {
        let Some(index) = line_index.checked_sub(distance) else {
            break;
        };
        let text = lines[index].trim();
        if text.is_empty() {
            break;
        }
        let heading = text.trim_start_matches('#').trim();
        if let Some(letter) = explicit_type_label(heading)
            && heading.strip_prefix("Type:").is_some_and(|rest| {
                let rest = rest.trim();
                rest.len() == 1 && rest.as_bytes()[0].is_ascii_alphabetic()
            })
        {
            return Some(letter);
        }
        if distance > 0 && (text.starts_with('#') || explicit_type_label(text).is_some()) {
            break;
        }
    }
    None
}

// The declaration may be in a different chunk from the supporting fact.
// Identity comes only from deterministic declarations in the full normalized
// document; fact evidence IDs remain local to the current source window.
fn parse_segmented_window_with_identity(
    response: &str,
    source: &str,
    identity_source: &str,
) -> Result<ExtractedRecords, String> {
    // Models occasionally emit both the legacy source_anchor and segment_id
    // for the same fact. Serde treats these aliases as a duplicate field.
    // Accept only an identical pair; conflicting locators remain invalid.
    let mut value: serde_json::Value =
        serde_json::from_str(strip_code_fences(response.trim()).trim())
            .map_err(|err| format!("Invalid segmented extraction JSON: {err}"))?;
    if let Some(files) = value
        .get_mut("files")
        .and_then(serde_json::Value::as_array_mut)
    {
        for file in files {
            if let Some(facts) = file
                .get_mut("facts")
                .and_then(serde_json::Value::as_array_mut)
            {
                for fact in facts {
                    if let Some(fields) = fact.as_object_mut()
                        && let (Some(anchor), Some(segment)) =
                            (fields.get("source_anchor"), fields.get("segment_id"))
                    {
                        if anchor != segment {
                            return Err("Conflicting source_anchor and segment_id".into());
                        }
                        fields.remove("source_anchor");
                    }
                }
            }
        }
    }
    let parsed: SegmentedWindow = serde_json::from_value(value)
        .map_err(|err| format!("Invalid segmented extraction JSON: {err}"))?;
    let segments = evidence_segments(source);
    let mut result: ExtractedRecords = Vec::new();
    for file in parsed.files {
        if !valid_spec_path(&file.path) {
            return Err(format!("Invalid specification path: {}", file.path));
        }
        // Normalize the common model spelling "type_g" only when the source
        // explicitly declares Type: G. Never infer identity from the filename.
        let mut path = file.path.clone();
        if let Some(slug) = path
            .strip_prefix("messages/type_")
            .and_then(|name| name.strip_suffix(".md"))
            && slug.len() == 1
            && slug.bytes().all(|byte| byte.is_ascii_lowercase())
        {
            let candidate = format!("messages/{slug}.md");
            let declared = declared_message_identity(&candidate, identity_source);
            if declared {
                path = candidate;
            }
        }
        let mut identity_id = file.identity_segment_id.as_deref();
        for fact in &file.facts {
            if let Some(id) = fact.identity_segment_id.as_deref() {
                if identity_id.is_some_and(|existing| existing != id) {
                    return Err(format!(
                        "Conflicting identity segment IDs for {}",
                        file.path
                    ));
                }
                identity_id = Some(id);
            }
        }
        if path.starts_with("messages/")
            && !declared_message_identity(&path, identity_source)
            && !identity_id
                .and_then(|id| segments.iter().find(|segment| segment.id == id))
                .is_some_and(|segment| declared_message_identity(&path, &segment.text))
        {
            return Err(format!(
                "Missing or invalid source-declared identity for {}",
                file.path
            ));
        }
        for fact in file.facts {
            let statement = fact.statement.trim();
            if statement.is_empty() || statement.chars().count() > 4_000 {
                return Err("Empty or oversized extracted fact".into());
            }
            if let (Some(actual), Some(expected)) = (
                explicit_type_label(statement),
                message_path_type(&path, identity_source),
            ) && actual != expected
            {
                return Err(format!(
                    "Fact declares Type: {} but destination {} requires Type: {}",
                    actual as char, path, expected as char
                ));
            }
            let quote = segments
                .iter()
                .find(|segment| segment.id == fact.segment_id)
                .map(|segment| segment.text.clone())
                .filter(|text| !text.trim().is_empty());
            // A verbatim source segment can prove that a fact belongs to a
            // different message even when the model omits the Type: prefix
            // from its statement. Do not infer ownership from generic rows.
            if let (Some(evidence_type), Some(expected)) = (
                segments
                    .iter()
                    .find(|segment| segment.id == fact.segment_id)
                    .and_then(|segment| {
                        explicit_type_label(&segment.text)
                            .or_else(|| section_type_for_line(source, segment.line_index))
                    }),
                message_path_type(&path, identity_source),
            ) && evidence_type != expected
            {
                return Err(format!(
                    "Evidence declares Type: {} but destination {} requires Type: {}",
                    evidence_type as char, path, expected as char
                ));
            }
            let evidence_type = segments
                .iter()
                .find(|segment| segment.id == fact.segment_id)
                .and_then(|segment| {
                    explicit_type_label(&segment.text)
                        .or_else(|| section_type_for_line(source, segment.line_index))
                });
            let statement_type = explicit_type_label(statement);
            if let (Some(a), Some(b)) = (statement_type, evidence_type)
                && a != b
            {
                return Err(format!(
                    "Fact Type: {} contradicts evidence Type: {}",
                    a as char, b as char
                ));
            }
            let destination =
                fact_message_path(&path, statement_type.or(evidence_type), identity_source)?;
            if let Some((_, existing)) = result
                .iter_mut()
                .find(|(existing_path, _)| existing_path == &destination)
            {
                existing.push((statement.to_owned(), quote));
            } else {
                result.push((destination, vec![(statement.to_owned(), quote)]));
            }
        }
    }
    Ok(result)
}

// Isolate semantically invalid file entries only when the outer JSON is
// complete and valid. This avoids retrying an entire window because one
// hallucinated identity was mixed with legitimate facts. The caller MUST
// mark the resulting output unverified when any entry was discarded.
fn parse_segmented_window_recoverable(
    response: &str,
    source: &str,
    identity_source: &str,
) -> Result<(ExtractedRecords, usize), String> {
    if let Ok(records) = parse_segmented_window_with_identity(response, source, identity_source) {
        return Ok((records, 0));
    }
    let normalized = strip_code_fences(response.trim());
    let root: serde_json::Value = serde_json::from_str(normalized.trim())
        .map_err(|err| format!("Invalid segmented extraction JSON: {err}"))?;
    let object = root
        .as_object()
        .ok_or("Segmented extraction must be an object")?;
    if object.len() != 1 || !object.contains_key("files") {
        return Err("Segmented extraction must contain only files".into());
    }
    let files = object["files"]
        .as_array()
        .ok_or("Segmented extraction files must be an array")?;
    let mut records = Vec::new();
    let mut rejected = 0;
    for file in files {
        let isolated = serde_json::json!({"files": [file]});
        match parse_segmented_window_with_identity(&isolated.to_string(), source, identity_source) {
            Ok(found) => records.extend(found),
            Err(reason) => {
                // Only split a structurally valid file. Malformed fields,
                // paths and conflicting identity declarations are file-level
                // failures, not permission to bypass validation.
                let parsed_file = serde_json::from_value::<SegmentedFile>(file.clone());
                if let Ok(parsed_file) = parsed_file {
                    let mut identity = parsed_file.identity_segment_id.as_deref();
                    let mut conflicting = false;
                    for fact in &parsed_file.facts {
                        if let Some(id) = fact.identity_segment_id.as_deref() {
                            if identity.is_some_and(|previous| previous != id) {
                                conflicting = true;
                                break;
                            }
                            identity = Some(id);
                        }
                    }
                    if !conflicting && !parsed_file.facts.is_empty() {
                        let mut accepted = 0;
                        for fact in &parsed_file.facts {
                            let isolated = serde_json::json!({"files": [{
                                "path": parsed_file.path,
                                "identity_segment_id": parsed_file.identity_segment_id,
                                "facts": [fact],
                            }]});
                            match parse_segmented_window_with_identity(
                                &isolated.to_string(),
                                source,
                                identity_source,
                            ) {
                                Ok(found) => {
                                    records.extend(found);
                                    accepted += 1;
                                }
                                Err(fact_reason) => {
                                    rejected += 1;
                                    println!("## [CHUNKED] REJECT FACT: {fact_reason}");
                                }
                            }
                        }
                        if accepted > 0 {
                            continue;
                        }
                        // All facts were rejected; the count already records
                        // the loss and must not be incremented a second time.
                        continue;
                    }
                }
                rejected += 1;
                println!("## [CHUNKED] REJECT FILE: {reason}");
            }
        }
    }
    // A complete JSON response with only rejected entries is still a
    // completed extraction window, not a transport/JSON failure. The caller
    // records the loss as unresolved and continues with the next window.
    // Otherwise a hallucinated identity can trigger an expensive split tree.
    Ok((records, rejected))
}

fn unsupported_quote_count(records: &ExtractedRecords) -> usize {
    records
        .iter()
        .flat_map(|(_, entries)| entries)
        .filter(|(_, quote)| quote.is_none())
        .count()
}

#[cfg(test)]
fn parse_structured_window(response: &str, source: &str) -> Result<ExtractedRecords, String> {
    let normalized = strip_code_fences(response.trim());
    let parsed: ExtractedWindow = serde_json::from_str(normalized.trim())
        .map_err(|e| format!("Invalid structured extraction JSON: {e}"))?;
    let mut result = Vec::new();
    for file in parsed.files {
        if !valid_spec_path(&file.path) {
            return Err(format!("Invalid specification path: {}", file.path));
        }
        if file.facts.is_empty() {
            continue;
        }
        let mut entries = Vec::new();
        for fact in file.facts {
            let statement = fact.statement.trim();
            if statement.is_empty() || statement.chars().count() > 4_000 {
                return Err("Empty or oversized extracted fact".into());
            }
            // The model supplies a locator, never the authoritative evidence.
            // Rust copies the matching bytes from the original source window.
            // Keep accepting evidence_quote from earlier model responses.
            let quote = fact
                .source_anchor
                .as_deref()
                .and_then(|anchor| resolve_source_quote(source, anchor))
                .filter(|matched| source.match_indices(matched).nth(1).is_none())
                .or_else(|| {
                    fact.evidence_quote
                        .as_deref()
                        .and_then(|quote| resolve_source_quote(source, quote))
                });
            entries.push((statement.to_owned(), quote));
        }
        result.push((file.path, entries));
    }
    Ok(result)
}

// A segment locator proves that source text exists, not that the model's
// assertion is semantically supported. Always require the later fact audit.
fn record_segmented_facts(
    facts: &mut BTreeMap<String, Vec<SourceFact>>,
    path: &str,
    entries: Vec<(String, Option<String>)>,
    chunk: usize,
    start_line: usize,
    end_line: usize,
) {
    let destination = facts.entry(path.to_owned()).or_default();
    for (statement, quote) in entries {
        if let Some(existing) = destination
            .iter_mut()
            .find(|fact| fact.statement == statement)
        {
            let occurrence = FactEvidenceOccurrence {
                source_chunk: chunk,
                start_line,
                end_line,
                evidence_quote: quote.clone(),
                source_byte_offset: None,
            };
            if !existing.evidence_occurrences.contains(&occurrence) {
                existing.evidence_occurrences.push(occurrence);
            }
            // A later source locator may replace a missing one, but never
            // silently turn an unreviewed assertion into a supported fact.
            if existing.evidence_quote.is_none()
                && let Some(quote) = quote
            {
                existing.evidence_quote = Some(quote);
                existing.source_chunk = chunk;
                existing.start_line = start_line;
                existing.end_line = end_line;
                existing.source_byte_offset = None;
                existing.review = FactReview::Pending;
            }
            continue;
        }
        destination.push(SourceFact {
            statement,
            source_chunk: chunk,
            start_line,
            end_line,
            source_byte_offset: None,
            evidence_occurrences: vec![FactEvidenceOccurrence {
                source_chunk: chunk,
                start_line,
                end_line,
                evidence_quote: quote.clone(),
                source_byte_offset: None,
            }],
            evidence_quote: quote,
            review: FactReview::Pending,
        });
    }
}

#[cfg(test)]
fn record_verified_facts(
    facts: &mut BTreeMap<String, Vec<SourceFact>>,
    path: &str,
    entries: Vec<(String, Option<String>)>,
    chunk: usize,
    start_line: usize,
    end_line: usize,
) {
    let destination = facts.entry(path.to_owned()).or_default();
    for (statement, quote) in entries {
        if let Some(existing) = destination
            .iter_mut()
            .find(|fact| fact.statement == statement)
        {
            if existing.review != FactReview::Supported
                && let Some(quote) = quote
            {
                existing.evidence_quote = Some(quote);
                existing.review = FactReview::Supported;
                // The accepted evidence belongs to this window, not to the
                // earlier occurrence of the same unsupported assertion.
                existing.source_chunk = chunk;
                existing.start_line = start_line;
                existing.end_line = end_line;
                existing.source_byte_offset = None;
            }
            continue;
        }
        let review = if quote.is_some() {
            FactReview::Supported
        } else {
            FactReview::Unresolved
        };
        destination.push(SourceFact {
            statement,
            source_chunk: chunk,
            start_line,
            end_line,
            evidence_quote: quote,
            source_byte_offset: None,
            evidence_occurrences: Vec::new(),
            review,
        });
    }
}

#[cfg(test)]
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
                source_byte_offset: None,
                evidence_occurrences: Vec::new(),
                review: FactReview::Pending,
            });
        }
    }
}

/// Record a P4 occurrence with its precise source position, not the full
/// overlapping window line range. Keep older checkpoint fields compatible.
fn record_positioned_candidate_fact(
    facts: &mut BTreeMap<String, Vec<SourceFact>>,
    path: &str,
    statement: String,
    quote: String,
    source_chunk: usize,
    part: &Chunk,
    offset: Option<usize>,
) {
    let (start_line, end_line) = if let Some(offset) = offset {
        let prefix = &part.text[..offset];
        let start_line = part.start_line + prefix.bytes().filter(|b| *b == b'\n').count();
        let end_line = start_line + quote.bytes().filter(|b| *b == b'\n').count()
            - usize::from(quote.ends_with('\n'));
        (start_line, end_line.max(start_line))
    } else {
        (part.start_line, part.end_line)
    };
    let occurrence = FactEvidenceOccurrence {
        source_chunk,
        start_line,
        end_line,
        evidence_quote: Some(quote.clone()),
        source_byte_offset: offset,
    };
    let destination = facts.entry(path.to_owned()).or_default();
    if let Some(existing) = destination
        .iter_mut()
        .find(|fact| fact.statement == statement)
    {
        // Exact position is part of occurrence identity: two quotes on the
        // same line must not overwrite each other's source byte offsets.
        if !existing.evidence_occurrences.contains(&occurrence) {
            existing.evidence_occurrences.push(occurrence);
        }
        if existing.evidence_quote.is_none() {
            existing.evidence_quote = Some(quote);
            existing.source_chunk = source_chunk;
            existing.start_line = start_line;
            existing.end_line = end_line;
            existing.source_byte_offset = offset;
            existing.review = FactReview::Pending;
        }
        return;
    }
    destination.push(SourceFact {
        statement,
        source_chunk,
        start_line,
        end_line,
        evidence_quote: Some(quote),
        source_byte_offset: offset,
        evidence_occurrences: vec![occurrence],
        review: FactReview::Pending,
    });
}

/// A reviewed assertion may only be reused for the exact same positioned
/// occurrence. Legacy window-only locations deliberately cannot share reviews.
fn same_source_occurrence_for_audit(previous: &SourceFact, current: &SourceFact) -> bool {
    current.source_byte_offset.is_some()
        && previous.source_byte_offset == current.source_byte_offset
        && previous.source_chunk == current.source_chunk
        && previous.statement == current.statement
        && previous.review != FactReview::Pending
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
fn canonical_message_path(path: &str, fragment: &str) -> Result<String, String> {
    if !path.starts_with("messages/") {
        return Ok(path.to_owned());
    }
    let slug = path
        .strip_prefix("messages/")
        .and_then(|name| name.strip_suffix(".md"))
        .unwrap_or("");
    // Preserve an already resolved case-sensitive identity. Never collapse
    // messages/upper_a.md into messages/a.md during checkpoint merging.
    let filename_identity = if let Some(letter) = slug.strip_prefix("upper_") {
        (letter.len() == 1 && letter.bytes().all(|b| b.is_ascii_lowercase()))
            .then(|| letter.as_bytes()[0].to_ascii_uppercase())
    } else if slug.len() == 1 && slug.bytes().all(|b| b.is_ascii_lowercase()) {
        Some(slug.as_bytes()[0])
    } else {
        ["_message", "_command"].iter().find_map(|suffix| {
            let letter = slug.strip_suffix(suffix)?;
            (letter.len() == 1 && letter.bytes().all(|b| b.is_ascii_lowercase()))
                .then(|| letter.as_bytes()[0])
        })
    };
    let mut heading_identity: Option<u8> = None;
    for line in fragment.lines().take(12) {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            continue;
        }
        let heading = trimmed.trim_start_matches('#').trim();
        for prefix in ["message ", "command ", "message: ", "command: "] {
            if heading
                .get(..prefix.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
            {
                let token = heading[prefix.len()..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c: char| !c.is_ascii_alphanumeric());
                if token.len() == 1 && token.as_bytes()[0].is_ascii_alphabetic() {
                    let letter = token.as_bytes()[0];
                    if heading_identity.is_some_and(|old| old != letter) {
                        return Err(format!("Conflicting message headings in {path}"));
                    }
                    heading_identity = Some(letter);
                }
                break;
            }
        }
    }
    if let (Some(filename), Some(heading)) = (filename_identity, heading_identity)
        && filename != heading
    {
        return Err(format!(
            "Conflicting message identity in {path}: filename {}, heading {}",
            filename as char, heading as char
        ));
    }
    match filename_identity.or(heading_identity) {
        Some(identity) if identity.is_ascii_uppercase() => Ok(format!(
            "messages/upper_{}.md",
            (identity as char).to_ascii_lowercase()
        )),
        Some(identity) => Ok(format!("messages/{}.md", identity as char)),
        None => Ok(path.to_owned()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditCheckpoint {
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
    output.with_extension("v10.checkpoint.json")
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
        || checkpoint
            .recovery_chunk
            .is_some_and(|index| index != checkpoint.next_chunk || index >= total_chunks)
        || (checkpoint.recovery_chunk.is_none()
            && (!checkpoint.recovery_pending.is_empty()
                || !checkpoint.recovery_records.is_empty()
                || !checkpoint.recovery_candidates.is_empty()))
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
    /// Absolute byte position in the normalized document.
    pub start_byte: usize,
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
            start_byte,
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

/// Recovery-only windows use a smaller overlap than primary chunks to ensure
/// subdivision actually reduces the amount of work and always makes progress.
fn recovery_chunks(source: &str, limit: usize) -> Vec<String> {
    let offsets: Vec<usize> = source
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once(source.len()))
        .collect();
    let total = offsets.len() - 1;
    let overlap = (limit / 4).min(256);
    let mut start = 0;
    let mut result = Vec::new();
    while start < total {
        let end = (start + limit).min(total);
        result.push(source[offsets[start]..offsets[end]].to_owned());
        if end == total {
            break;
        }
        start = end - overlap;
    }
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

/// The output is a deterministic JSON projection of the canonical facts.
/// Source facts may still contain Markdown-formatted statements from the LLM.
#[derive(Serialize)]
struct JsonSpecification<'a> {
    schema_version: u32,
    name: &'a str,
    facts: &'a [SourceFact],
    source_windows: Vec<usize>,
    verification: &'static str,
}

#[derive(Serialize)]
struct JsonManifest<'a> {
    schema_version: u32,
    source: String,
    verification: &'static str,
    files: Vec<String>,
    facts_file: &'a str,
}

/// Stage JSON output in a sibling directory and publish it atomically.
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
        let mut output_paths = Vec::new();
        for path in files.keys() {
            if !valid_spec_path(path) {
                return Err(format!("Invalid specification path: {path}"));
            }
            let json_path = path
                .strip_suffix(".md")
                .ok_or_else(|| format!("Invalid specification extension: {path}"))?
                .to_owned()
                + ".json";
            let entries = facts.get(path).map(Vec::as_slice).unwrap_or(&[]);
            let mut windows = source_windows.get(path).cloned().unwrap_or_default();
            windows.sort_unstable();
            windows.dedup();
            let verification = if entries
                .iter()
                .any(|fact| fact.review != FactReview::Supported)
            {
                "unresolved"
            } else if entries.is_empty() {
                "no_facts"
            } else {
                "supported_quotes"
            };
            let document = JsonSpecification {
                schema_version: 1,
                name: path.strip_suffix(".md").unwrap_or(path),
                facts: entries,
                source_windows: windows.into_iter().map(|i| i + 1).collect(),
                verification,
            };
            let bytes = serde_json::to_vec_pretty(&document)
                .map_err(|e| format!("Cannot serialize {json_path}: {e}"))?;
            fs::write(stage.join(&json_path), bytes)
                .map_err(|e| format!("Cannot write {json_path}: {e}"))?;
            output_paths.push(json_path);
        }
        let fact_json =
            serde_json::to_vec_pretty(facts).map_err(|e| format!("Cannot serialize facts: {e}"))?;
        fs::write(stage.join("facts.json"), fact_json)
            .map_err(|e| format!("Cannot write facts.json: {e}"))?;
        let manifest = JsonManifest {
            schema_version: 1,
            source: source.display().to_string(),
            verification: if unverified { "unverified" } else { "reviewed" },
            files: output_paths,
            facts_file: "facts.json",
        };
        let manifest_json = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| format!("Cannot serialize manifest: {e}"))?;
        fs::write(stage.join("manifest.json"), manifest_json)
            .map_err(|e| format!("Cannot write manifest.json: {e}"))?;
        fs::rename(&stage, output).map_err(|e| format!("Cannot publish output: {e}"))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}

// The runtime prompt contracts are fingerprinted together with the source.
const JSON_EXTRACT_PROMPT: &str = r#"Extract concise atomic protocol facts from numbered SOURCE segments. Return exactly one JSON object: {"files":[{"path":"general.md","facts":[{"statement":"short assertion","segment_id":"S1"}]}]}. Allowed paths: general.md, infrastructure.md, messages/<lowercase-slug>.md. For a messages/ file, include "identity_segment_id":"S2" ONLY when a local SOURCE segment explicitly declares its message name (for example Type: A). If the declaration is in another chunk, omit identity_segment_id; the workflow verifies identity against the full document. Every fact must cite one supporting segment_id. Never invent IDs. Put identity_segment_id ONLY on each file, NEVER inside facts. Do not invent paths such as messages/type_g.md when Type: G means messages/g.md. Keep uppercase and lowercase one-letter message types separate: when both X and x exist, write X to messages/upper_x.md and x to messages/x.md. Omit unsupported facts. Empty: {"files":[]}. No copied quotes, Markdown fences or commentary."#;
const JSON_COVERAGE_PROMPT: &str = r#"Independently discover missing protocol messages from numbered SOURCE segments. Return exactly one JSON object: {"files":[{"path":"messages/<lowercase-slug>.md","identity_segment_id":"S1","facts":[{"statement":"short atomic assertion","segment_id":"S2"}]}]}. Empty: {"files":[]}. Include identity_segment_id only when a local SOURCE segment explicitly declares the message, not merely mentions it. Omit it when the declaration is elsewhere; the workflow validates the identity against the full document. Use existing segment IDs only. Put identity_segment_id ONLY on each file, NEVER inside facts. For one-letter message types, preserve case: when both X and x are declared, use messages/upper_x.md for X and messages/x.md for x. Never mix their facts. A Type: G row identifies messages/g.md, not messages/type_g.md. No general.md, infrastructure.md, copied quotes, Markdown or commentary."#;

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
        let output = input.with_file_name(format!("{stem}_messages-json"));
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
        let masked_source =
            normalize_html_tables(&mask_opaque_lines(&mask_embedded_data_uris(&source)));
        let parts = chunks(&masked_source, CHUNK_CHARS);
        if parts.is_empty() {
            return WorkflowResult::Error("Input document is empty".into());
        }

        // Each review receives the original source excerpt and the draft,
        // never the entire document or an unbounded collection of drafts.
        let requirements = Tasks::ExtractMessageTables.get_prompt();
        // New checkpoint format; previous extraction checkpoints are never loaded.
        let checkpoint_output = output.with_file_name(
            output
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .replace("_messages-json", "_messages"),
        );
        let checkpoint_file = checkpoint_path(&checkpoint_output);
        let fingerprint = extraction_fingerprint(
            &source,
            &[
                requirements,
                JSON_EXTRACT_PROMPT,
                JSON_COVERAGE_PROMPT,
                "segmented-evidence-v12-deferred-candidates",
            ],
        );
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
                identity_projection_version: 0,
                recovery_chunk: None,
                recovery_pending: Vec::new(),
                recovery_records: Vec::new(),
                candidates: Vec::new(),
                recovery_candidates: Vec::new(),
            },
            Err(err) => return WorkflowResult::Error(err),
        };
        if std::env::var_os("AIFIX_IDENTITY_REPLAY").is_some()
            && (checkpoint.next_chunk != parts.len()
                || checkpoint.coverage_next_chunk != parts.len()
                || checkpoint.recovery_chunk.is_some()
                || !checkpoint.recovery_pending.is_empty())
        {
            return WorkflowResult::Error(
                "Identity replay requires a completed extraction and coverage checkpoint; no LLM calls were made".into(),
            );
        }
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
            let json_prompt = format!("{requirements}\n\n{}", JSON_EXTRACT_PROMPT);
            // Each source window is committed only after all recovery windows succeed.
            // EOF is usually a truncated model response: retrying the identical
            // deterministic request cannot make progress. Subdivide instead.
            let mut pending = if checkpoint.recovery_chunk == Some(index) {
                VecDeque::from(checkpoint.recovery_pending.clone())
            } else {
                VecDeque::from([part.text.clone()])
            };
            let mut records: ExtractedRecords = if checkpoint.recovery_chunk == Some(index) {
                checkpoint.recovery_records.clone()
            } else {
                Vec::new()
            };
            let mut staged_candidates = if checkpoint.recovery_chunk == Some(index) {
                checkpoint.recovery_candidates.clone()
            } else {
                Vec::new()
            };
            let mut recovery_windows = 0_usize;
            while let Some(window) = pending.pop_front() {
                recovery_windows += 1;
                if recovery_windows > MAX_RECOVERY_WINDOWS {
                    return WorkflowResult::Error(format!(
                        "Chunk {} exceeded {MAX_RECOVERY_WINDOWS} recovery windows",
                        index + 1
                    ));
                }
                let preceding = preceding_extraction_context(&masked_source, part, &window);
                let window_context = if window == part.text {
                    format!(
                        "{preceding}{}",
                        context.replace(&part.text, &segmented_source(&window))
                    )
                } else {
                    format!(
                        "{preceding}SOURCE: {}\nCHUNK: {}/{} (recovery window)\nLINES: {}-{}\n\n=== SOURCE ===\n{}",
                        input.display(),
                        index + 1,
                        parts.len(),
                        part.start_line,
                        part.end_line,
                        segmented_source(&window)
                    )
                };
                let response = match self.llm_call.run_text_step_limited_with_temperature(
                    &json_prompt,
                    &window_context,
                    MAX_TOKENS * 2,
                    0.0,
                ) {
                    Ok(text) => text,
                    Err(err) => return WorkflowResult::LlmCallResult(err),
                };
                let parsed = parse_segmented_window_recoverable(&response, &window, &masked_source);
                if let Err(reason) = &parsed {
                    let normalized = strip_code_fences(response.trim());
                    let eof = serde_json::from_str::<SegmentedWindow>(&normalized)
                        .is_err_and(|err| err.is_eof());
                    println!(
                        "## [CHUNKED] EXTRACT INVALID {}/{}: source_chars={}, response_chars={}, eof={}, recovery_window={}; {}",
                        index + 1,
                        parts.len(),
                        window.chars().count(),
                        response.chars().count(),
                        eof,
                        recovery_windows,
                        reason
                    );
                }
                let parsed = match parsed {
                    Ok(found) => Ok((found, response.clone())),
                    Err(reason) => {
                        let is_eof = serde_json::from_str::<SegmentedWindow>(
                            strip_code_fences(response.trim()).trim(),
                        )
                        .is_err_and(|err| err.is_eof());
                        // A truncated response is worth one explicit compact
                        // retry before recursively splitting the source. This
                        // also applies to the smallest recovery windows, where
                        // another split is impossible.
                        {
                            println!(
                                "## [CHUNKED] EXTRACT RETRY {}/{}: {reason}",
                                index + 1,
                                parts.len()
                            );
                            let retry_prompt = if is_eof {
                                format!(
                                    "{json_prompt}\n\nPrevious JSON was truncated. Return a COMPLETE, COMPACT JSON object with a closed files array. Prefer fewer facts over incomplete JSON; do not invent or omit closing delimiters."
                                )
                            } else {
                                format!(
                                    "{json_prompt}\n\nPrevious response was invalid: {reason}. Return valid JSON only."
                                )
                            };
                            let retried =
                                match self.llm_call.run_text_step_limited_with_temperature(
                                    &retry_prompt,
                                    &window_context,
                                    MAX_TOKENS * 2,
                                    0.0,
                                ) {
                                    Ok(text) => text,
                                    Err(err) => return WorkflowResult::LlmCallResult(err),
                                };
                            parse_segmented_window_recoverable(&retried, &window, &masked_source)
                                .map(|found| (found, retried))
                        }
                    }
                };
                match parsed {
                    Ok(((found, rejected), accepted_response)) => {
                        let next_id = staged_candidates
                            .iter()
                            .filter_map(|candidate| {
                                candidate.id.rsplit('-').next()?.parse::<usize>().ok()
                            })
                            .max()
                            .map_or(0, |last| last + 1);
                        let staged = message_identity::parse_candidates_in_parent(
                            &accepted_response,
                            &window,
                            &part.text,
                            index,
                            next_id,
                        );
                        staged_candidates.extend(staged);
                        if rejected > 0 {
                            checkpoint.unresolved_findings = true;
                            println!(
                                "## [CHUNKED] EXTRACT PARTIAL {}/{}: {rejected} invalid file entries discarded",
                                index + 1,
                                parts.len()
                            );
                        }
                        records.extend(found);
                    }
                    Err(reason) => {
                        let length = window.chars().count();
                        if length <= MIN_RECOVERY_CHARS {
                            return WorkflowResult::Error(format!(
                                "Chunk {} failed even at {length} characters: {reason}",
                                index + 1
                            ));
                        }
                        let limit = length.div_ceil(2).max(MIN_RECOVERY_CHARS);
                        let children = recovery_chunks(&window, limit);
                        if children.len() < 2 || children.iter().any(|child| child == &window) {
                            return WorkflowResult::Error(format!(
                                "Chunk {} could not be subdivided: {reason}",
                                index + 1
                            ));
                        }
                        println!(
                            "## [CHUNKED] EXTRACT SPLIT {}/{}: {length} chars -> {} windows ({reason})",
                            index + 1,
                            parts.len(),
                            children.len()
                        );
                        for child in children.into_iter().rev() {
                            pending.push_front(child);
                        }
                    }
                }
                // Persist progress after each successful subwindow or split. A
                // restart never needs to replay already verified subwindows.
                checkpoint.recovery_chunk = Some(index);
                checkpoint.recovery_pending = pending.iter().cloned().collect();
                checkpoint.recovery_records = records.clone();
                checkpoint.recovery_candidates = staged_candidates.clone();
                if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                    return WorkflowResult::Error(err);
                }
            }
            checkpoint.candidates.extend(staged_candidates);
            let unsupported = unsupported_quote_count(&records);
            if unsupported > 0 {
                println!(
                    "## [CHUNKED] EXTRACT UNRESOLVED {}/{}: {unsupported} assertions lack source quotes; retained for audit",
                    index + 1,
                    parts.len()
                );
                // The fact audit decides whether these assertions remain
                // unresolved; a missing locator alone is not a permanent warning.
            }
            for (model_path, entries) in records {
                let fragment = entries
                    .iter()
                    .map(|(statement, _)| statement.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let path = match canonical_message_path(&model_path, &fragment) {
                    Ok(path) => path,
                    Err(reason) => {
                        println!("## [CHUNKED] IDENTITY WARNING: {reason}");
                        checkpoint.unresolved_findings = true;
                        // Do not publish facts under a disputed identity.
                        // The source window and checkpoint remain available
                        // for inspection and a corrected extraction.
                        continue;
                    }
                };
                let source_windows = checkpoint.source_windows.entry(path.clone()).or_default();
                if !source_windows.contains(&index) {
                    source_windows.push(index);
                }
                record_segmented_facts(
                    &mut checkpoint.facts,
                    &path,
                    entries,
                    index,
                    part.start_line,
                    part.end_line,
                );
                files.insert(path.clone(), render_source_facts(&checkpoint.facts[&path]));
            }
            checkpoint.recovery_chunk = None;
            checkpoint.recovery_pending.clear();
            checkpoint.recovery_records.clear();
            checkpoint.recovery_candidates.clear();
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
        let coverage_prompt = format!("{requirements}\n\n{}", JSON_COVERAGE_PROMPT);
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
                println!("## [CHUNKED] COVERAGE {}/{}", index + 1, parts.len());
                let mut pending = VecDeque::from([part.text.clone()]);
                let mut records = Vec::new();
                let mut coverage_candidates = Vec::new();
                let mut windows_seen = 0;
                while let Some(window) = pending.pop_front() {
                    windows_seen += 1;
                    if windows_seen > MAX_RECOVERY_WINDOWS {
                        println!(
                            "## [CHUNKED] COVERAGE WARNING {}/{}: exceeded {MAX_RECOVERY_WINDOWS} recovery windows; discovery incomplete",
                            index + 1,
                            parts.len()
                        );
                        checkpoint.unresolved_findings = true;
                        break;
                    }
                    let preceding = preceding_extraction_context(&masked_source, part, &window);
                    let context = format!(
                        "{preceding}SOURCE CHUNK {}/{} LINES {}-{}\n\n=== KNOWN MESSAGES ===\n{inventory}\n\n=== SOURCE ===\n{}",
                        index + 1,
                        parts.len(),
                        part.start_line,
                        part.end_line,
                        segmented_source(&window)
                    );
                    if context.chars().count() > MAX_MERGE_CHARS {
                        return WorkflowResult::Error(format!(
                            "Coverage context exceeds {MAX_MERGE_CHARS} characters at chunk {}",
                            index + 1
                        ));
                    }
                    let response = match self.llm_call.run_text_step_limited_with_temperature(
                        &coverage_prompt,
                        &context,
                        MAX_TOKENS,
                        0.0,
                    ) {
                        Ok(text) => text,
                        Err(err) => return WorkflowResult::LlmCallResult(err),
                    };
                    let parsed = match parse_segmented_window_recoverable(
                        &response,
                        &window,
                        &masked_source,
                    ) {
                        Ok(found) => Ok((found, response)),
                        Err(first_error) => {
                            let is_eof = serde_json::from_str::<SegmentedWindow>(
                                strip_code_fences(response.trim()).trim(),
                            )
                            .is_err_and(|err| err.is_eof());
                            if is_eof {
                                Err(first_error)
                            } else {
                                println!(
                                    "## [CHUNKED] COVERAGE RETRY {}/{}",
                                    index + 1,
                                    parts.len()
                                );
                                let retry_prompt = format!(
                                    "{coverage_prompt}\n\nPrevious JSON response was invalid: {first_error}. Return a single valid JSON object only."
                                );
                                let retried =
                                    match self.llm_call.run_text_step_limited_with_temperature(
                                        &retry_prompt,
                                        &context,
                                        MAX_TOKENS * 2,
                                        0.0,
                                    ) {
                                        Ok(text) => text,
                                        Err(err) => return WorkflowResult::LlmCallResult(err),
                                    };
                                parse_segmented_window_recoverable(
                                    &retried,
                                    &window,
                                    &masked_source,
                                )
                                .map(|found| (found, retried))
                            }
                        }
                    };
                    match parsed {
                        Ok(((found, rejected), accepted_response)) => {
                            let next_id = coverage_candidates
                                .iter()
                                .filter_map(|candidate: &message_identity::MessageCandidate| {
                                    candidate.id.rsplit('-').next()?.parse::<usize>().ok()
                                })
                                .max()
                                .map_or(0, |last| last + 1);
                            let mut staged = message_identity::parse_candidates_in_parent(
                                &accepted_response,
                                &window,
                                &part.text,
                                index,
                                next_id,
                            );
                            for candidate in &mut staged {
                                candidate.id.insert_str(0, "coverage-");
                            }
                            coverage_candidates.extend(staged);
                            if rejected > 0 {
                                checkpoint.unresolved_findings = true;
                                println!(
                                    "## [CHUNKED] COVERAGE PARTIAL {}/{}: {rejected} invalid file entries discarded",
                                    index + 1,
                                    parts.len()
                                );
                            }
                            records.extend(found);
                        }
                        Err(reason) => {
                            let length = window.chars().count();
                            if length <= MIN_RECOVERY_CHARS {
                                println!(
                                    "## [CHUNKED] COVERAGE WARNING {}/{}: invalid JSON at {length} characters ({reason}); discovery incomplete",
                                    index + 1,
                                    parts.len()
                                );
                                checkpoint.unresolved_findings = true;
                                continue;
                            }
                            let children = recovery_chunks(
                                &window,
                                length.div_ceil(2).max(MIN_RECOVERY_CHARS),
                            );
                            if children.len() < 2 || children.iter().any(|child| child == &window) {
                                println!(
                                    "## [CHUNKED] COVERAGE WARNING {}/{}: could not split window ({reason}); discovery incomplete",
                                    index + 1,
                                    parts.len()
                                );
                                checkpoint.unresolved_findings = true;
                                continue;
                            }
                            println!(
                                "## [CHUNKED] COVERAGE SPLIT {}/{}: {length} chars -> {} windows ({reason})",
                                index + 1,
                                parts.len(),
                                children.len()
                            );
                            for child in children.into_iter().rev() {
                                pending.push_front(child);
                            }
                        }
                    }
                }
                let staged_facts: usize = coverage_candidates
                    .iter()
                    .map(|candidate| candidate.facts.len())
                    .sum();
                println!(
                    "## [CHUNKED] COVERAGE CANDIDATES {}/{}: {} candidates, {} source-attributed facts",
                    index + 1,
                    parts.len(),
                    coverage_candidates.len(),
                    staged_facts
                );
                checkpoint.candidates.extend(coverage_candidates);
                for (model_path, entries) in records {
                    let fragment = entries
                        .iter()
                        .map(|(statement, _)| statement.as_str())
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    let path = match canonical_message_path(&model_path, &fragment) {
                        Ok(path) => path,
                        Err(reason) => {
                            println!("## [CHUNKED] IDENTITY WARNING: {reason}");
                            checkpoint.unresolved_findings = true;
                            // Coverage must not reintroduce facts rejected by
                            // deterministic identity validation.
                            continue;
                        }
                    };
                    if !coverage_message_path(&path) {
                        // Coverage is discovery-only. Non-message destinations
                        // must never modify general or infrastructure files,
                        // but must not trap resume on a deterministic model error.
                        println!("## [CHUNKED] COVERAGE IGNORE non-message path: {path}");
                        checkpoint.unresolved_findings = true;
                        continue;
                    }
                    if fragment.chars().count() > MAX_MERGE_CHARS {
                        return WorkflowResult::Error(format!(
                            "Coverage fragment for {path} exceeds {MAX_MERGE_CHARS} characters"
                        ));
                    }
                    record_segmented_facts(
                        &mut checkpoint.facts,
                        &path,
                        entries,
                        index,
                        part.start_line,
                        part.end_line,
                    );
                    let windows = checkpoint.source_windows.entry(path.clone()).or_default();
                    if !windows.contains(&index) {
                        windows.push(index);
                    }
                    files.insert(path.clone(), render_source_facts(&checkpoint.facts[&path]));
                }
            }
            checkpoint.coverage_next_chunk = index + 1;
            checkpoint.files = files.clone();
            if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                return WorkflowResult::Error(err);
            }
        }

        let candidate_facts: usize = checkpoint
            .candidates
            .iter()
            .map(|candidate| candidate.facts.len())
            .sum();
        println!(
            "## [CHUNKED] DEFERRED IDENTITY: {} candidates, {} source-attributed facts retained for later resolution",
            checkpoint.candidates.len(),
            candidate_facts
        );
        let distinct_candidate_facts =
            message_identity::distinct_source_fact_count(&checkpoint.candidates);
        println!(
            "## [CHUNKED] SOURCE FACT INVENTORY: {distinct_candidate_facts} distinct statement/evidence pairs from {candidate_facts} candidate facts; identities not merged"
        );
        let (resolved, unresolved, unresolved_facts) =
            message_identity::identity_counts(&checkpoint.candidates);
        println!(
            "## [CHUNKED] IDENTITY CLASSIFICATION: {resolved} explicitly typed candidates, {unresolved} unresolved candidates ({unresolved_facts} facts); no automatic alias merging"
        );
        let source_windows: Vec<&str> = parts.iter().map(|part| part.text.as_str()).collect();
        let source_starts: Vec<usize> = parts.iter().map(|part| part.start_byte).collect();
        let diagnostics = message_identity::resolution_diagnostics_in_document(
            &checkpoint.candidates,
            &source_windows,
            &source_starts,
            &masked_source,
        );
        println!(
            "## [CHUNKED] P4 UNRESOLVED DIAGNOSTICS: invalid_window={}, ambiguous_locator={}, missing_evidence={}, invalid_offset_or_quote={}, no_source_type={}, conflicting_type={}",
            diagnostics.invalid_window,
            diagnostics.ambiguous_locator,
            diagnostics.missing_evidence,
            diagnostics.invalid_offset_or_quote,
            diagnostics.no_source_type,
            diagnostics.conflicting_type,
        );
        let triage = message_identity::unresolved_triage_in_document(
            &checkpoint.candidates,
            &source_windows,
            &source_starts,
            &masked_source,
        );
        println!(
            "## [CHUNKED] P4 UNRESOLVED TRIAGE (STRUCTURAL HINTS): example_line={}, parameter_table_row={}, other={}, total_no_source_type={}; not semantic classifications",
            triage.example_line,
            triage.parameter_table_row,
            triage.other,
            diagnostics.no_source_type,
        );
        let parameter_inventory = message_identity::parameter_facts_in_document(
            &checkpoint.candidates,
            &source_windows,
            &source_starts,
            &masked_source,
        );
        println!(
            "## [CHUNKED] P4 PARAMETER INVENTORY: {} distinct source-backed parameter assertions; separate from message identity, not published",
            parameter_inventory.len()
        );
        // Explicit, read-only deterministic replay. This deliberately stops
        // before fact auditing and publication; neither invokes the model nor
        // mutates the checkpoint or an existing output directory.
        if std::env::var_os("AIFIX_IDENTITY_REPLAY").is_some() {
            for sample in message_identity::unresolved_history_samples(
                &checkpoint.candidates,
                &source_windows,
                &source_starts,
                &masked_source,
                30,
            ) {
                println!("## [CHUNKED] IDENTITY HISTORY UNRESOLVED: {sample}");
            }
            let (groups, unresolved) = message_identity::resolved_source_facts_in_document(
                &checkpoint.candidates,
                &source_windows,
                &source_starts,
                &masked_source,
            );
            let mut seen = std::collections::BTreeSet::new();
            let mut duplicates = 0_usize;
            let mut positioned = 0_usize;
            let mut projected = 0_usize;
            let mut by_message = BTreeMap::new();
            for (path, occurrences) in groups {
                for (window, fact) in occurrences {
                    let Some(part) = parts.get(window) else {
                        continue;
                    };
                    if let Some(offset) = fact.source_byte_offset {
                        let Some(absolute) = part.start_byte.checked_add(offset) else {
                            continue;
                        };
                        positioned += 1;
                        if !seen.insert((
                            path.clone(),
                            fact.statement.clone(),
                            absolute,
                            fact.evidence_quote.clone(),
                        )) {
                            duplicates += 1;
                            continue;
                        }
                    }
                    projected += 1;
                    *by_message.entry(path.clone()).or_insert(0_usize) += 1;
                }
            }
            println!(
                "## [CHUNKED] IDENTITY REPLAY (READ ONLY): projected={projected} duplicate_occurrences={duplicates} positioned={positioned} unresolved={unresolved} messages={}",
                by_message.len()
            );
            for (path, count) in by_message {
                println!("## [CHUNKED] IDENTITY REPLAY MESSAGE: {path} facts={count}");
            }
            return WorkflowResult::Ok;
        }
        if files.is_empty() {
            return WorkflowResult::Error("No protocol specifications were extracted".into());
        }
        files.entry("general.md".into()).or_insert_with(|| {
            "# General Protocol Rules\n\nNo shared rules were established from the source.".into()
        });
        files.entry("infrastructure.md".into()).or_insert_with(|| "# Infrastructure Contracts\n\nNo helper contracts were established from the source.".into());
        // P4: build the auditable projection from source-backed candidates
        // BEFORE reviewing facts. The model's proposed filenames are never
        // used as canonical identities. Existing non-message facts survive.
        // The version gate makes checkpoint resume idempotent.
        // Migrate already projected v7 checkpoints without replaying the
        // extraction or invalidating their completed fact review.
        migrate_identity_projection_version(&mut checkpoint);
        if checkpoint.identity_projection_version != 2 {
            let source_windows: Vec<&str> = parts.iter().map(|part| part.text.as_str()).collect();
            let (groups, mut unresolved) = message_identity::resolved_source_facts_in_document(
                &checkpoint.candidates,
                &source_windows,
                &source_starts,
                &masked_source,
            );
            checkpoint
                .facts
                .retain(|path, _| !path.starts_with("messages/"));
            files.retain(|path, _| !path.starts_with("messages/"));
            let mut accepted = 0_usize;
            let mut overlapping_duplicates = 0_usize;
            let mut seen_locations = std::collections::BTreeSet::new();
            for (path, occurrences) in groups {
                for (source_chunk, candidate_fact) in occurrences {
                    let Some(part) = parts.get(source_chunk) else {
                        unresolved += 1;
                        continue;
                    };
                    let Some(quote) = candidate_fact.evidence_quote else {
                        unresolved += 1;
                        continue;
                    };
                    let source_matches = if let Some(offset) = candidate_fact.source_byte_offset {
                        part.text.get(offset..offset.saturating_add(quote.len()))
                            == Some(quote.as_str())
                    } else {
                        // Checkpoints created before exact source positions.
                        part.text.contains(&quote)
                    };
                    if !source_matches {
                        unresolved += 1;
                        continue;
                    }
                    // Overlapping chunks may report the same physical source
                    // location. Deduplicate only exact positioned occurrences;
                    // legacy facts without offsets remain separate.
                    if let Some(offset) = candidate_fact.source_byte_offset {
                        let Some(absolute) = part.start_byte.checked_add(offset) else {
                            unresolved += 1;
                            continue;
                        };
                        if !seen_locations.insert((
                            path.clone(),
                            candidate_fact.statement.clone(),
                            absolute,
                            quote.clone(),
                        )) {
                            overlapping_duplicates += 1;
                            continue;
                        }
                    }
                    record_positioned_candidate_fact(
                        &mut checkpoint.facts,
                        &path,
                        candidate_fact.statement,
                        quote,
                        source_chunk,
                        part,
                        candidate_fact.source_byte_offset,
                    );
                    accepted += 1;
                }
            }
            if overlapping_duplicates > 0 {
                println!(
                    "## [CHUNKED] P4 OVERLAP DEDUP: {overlapping_duplicates} duplicate source occurrences"
                );
            }
            if unresolved > 0 {
                checkpoint.unresolved_findings = true;
            }
            let consolidated: usize = checkpoint
                .facts
                .iter()
                .filter(|(path, _)| path.starts_with("messages/"))
                .map(|(_, facts)| facts.len())
                .sum();
            println!(
                "## [CHUNKED] IDENTITY CONSOLIDATION: {accepted} source-backed candidate occurrences -> {consolidated} distinct typed message facts; {unresolved} unresolved candidate facts kept in checkpoint, not published"
            );
            checkpoint.identity_projection_version = 2;
            checkpoint.fact_audit_version = 8;
            checkpoint.fact_audit_cursor = 0;
            if checkpoint.audit.is_some() {
                checkpoint.unresolved_findings = true;
            }
            checkpoint.audit = None;
            // Persist the rebuilt projection too: resuming audit v2 must not
            // resurrect stale P3 message files from the checkpoint.
            checkpoint.files = files.clone();
            println!(
                "## [CHUNKED] RESET AUDIT: candidate consolidation before source-grounded fact review"
            );
            if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                return WorkflowResult::Error(err);
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
            // Facts may appear under several candidate identities. Reuse an
            // already completed evidence review only for the same statement
            // and original source window; never infer identity from this.
            // Restrict the lookup to earlier cursor entries so resume behavior
            // is deterministic and an unreviewed sibling cannot be promoted.
            let prior_review = fact_keys[..checkpoint.fact_audit_cursor]
                .iter()
                .filter_map(|(previous_path, previous_index)| {
                    let previous = &checkpoint.facts[previous_path][*previous_index];
                    same_source_occurrence_for_audit(previous, &fact)
                        .then_some((previous.review, previous.evidence_quote.clone()))
                })
                .find(|(review, quote)| {
                    *review == FactReview::Unresolved
                        || quote.as_ref().is_some_and(|quote| {
                            parts.get(fact.source_chunk).is_some_and(|part| {
                                quote.chars().count() >= 12 && part.text.contains(quote)
                            })
                        })
                });
            if let Some((review, quote)) = prior_review {
                let current =
                    &mut checkpoint.facts.get_mut(path).expect("fact path exists")[*index];
                current.review = review;
                current.evidence_quote = quote;
                checkpoint.fact_audit_cursor += 1;
                if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                    return WorkflowResult::Error(err);
                }
                continue;
            }
            if fact.review == FactReview::Supported
                && fact.evidence_quote.as_ref().is_some_and(|quote| {
                    parts.get(fact.source_chunk).is_some_and(|part| {
                        quote.chars().count() >= 12 && part.text.contains(quote)
                    })
                })
            {
                checkpoint.fact_audit_cursor += 1;
                if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                    return WorkflowResult::Error(err);
                }
                continue;
            }
            let source = parts.get(fact.source_chunk).map(|part| part.text.as_str());
            let result = if let Some(source) = source {
                if fact.statement.chars().count() > 4_000 || source.trim().is_empty() {
                    Err("Oversized fact or empty source window".to_owned())
                } else {
                    // Reuse original source bytes without another model call
                    // when the fact itself is a verbatim, unique source span.
                    let local_quote = resolve_source_quote(source, &fact.statement)
                        .filter(|quote| source.match_indices(quote).nth(1).is_none());
                    if let Some(quote) = local_quote {
                        let current =
                            &mut checkpoint.facts.get_mut(path).expect("fact path exists")[*index];
                        current.evidence_quote = Some(quote);
                        current.review = FactReview::Supported;
                        checkpoint.fact_audit_cursor += 1;
                        if let Err(err) = save_checkpoint(&checkpoint_file, &checkpoint) {
                            return WorkflowResult::Error(err);
                        }
                        continue;
                    }
                    println!(
                        "## [CHUNKED] FACT AUDIT {}/{}: {path} fact {}",
                        checkpoint.fact_audit_cursor + 1,
                        fact_keys.len(),
                        index + 1
                    );
                    let context = format!(
                        "DESTINATION: {path}\nSOURCE WINDOW: {} (lines {}-{})\n\n=== FACT ===\n{}\n\n=== ORIGINAL SOURCE ===\n{}\n",
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
                Err("Missing or unattributed original source window".to_owned())
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
        // A fact that never passed the audit must prevent verified publication.
        if checkpoint
            .facts
            .values()
            .flatten()
            .any(|fact| fact.review != FactReview::Supported)
        {
            checkpoint.unresolved_findings = true;
        }
        // JSON is the published projection of immutable extracted assertions. The
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
        // A completed unverified run retains its checkpoint. Re-running it
        // must be idempotent, but never silently accept stale or edited output.
        if destination.exists() {
            let expected_facts = match serde_json::to_value(&checkpoint.facts) {
                Ok(value) => value,
                Err(err) => return WorkflowResult::Error(err.to_string()),
            };
            let published_facts = fs::read(destination.join("facts.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
            let published_manifest = fs::read(destination.join("manifest.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
            let expected_paths = files
                .keys()
                .map(|path| path.trim_end_matches(".md").to_owned() + ".json")
                .collect::<Vec<_>>();
            let matching_manifest = published_manifest.as_ref().is_some_and(|manifest| {
                manifest.get("schema_version").and_then(|v| v.as_u64()) == Some(1)
                    && manifest.get("source").and_then(|v| v.as_str())
                        == Some(input.to_string_lossy().as_ref())
                    && manifest.get("verification").and_then(|v| v.as_str())
                        == Some(if checkpoint.unresolved_findings {
                            "unverified"
                        } else {
                            "reviewed"
                        })
                    && manifest.get("facts_file").and_then(|v| v.as_str()) == Some("facts.json")
                    && manifest
                        .get("files")
                        .and_then(|v| v.as_array())
                        .is_some_and(|paths| {
                            paths.iter().filter_map(|p| p.as_str()).collect::<Vec<_>>()
                                == expected_paths
                                    .iter()
                                    .map(String::as_str)
                                    .collect::<Vec<_>>()
                        })
                    && expected_paths
                        .iter()
                        .all(|path| destination.join(path).is_file())
            });
            if published_facts.as_ref() == Some(&expected_facts) && matching_manifest {
                println!(
                    "## [CHUNKED] OUTPUT ALREADY PUBLISHED {}",
                    destination.display()
                );
                return WorkflowResult::Ok;
            }
            return WorkflowResult::Error(format!(
                "Existing output differs from checkpoint or is incomplete: {}; preserved both output and checkpoint",
                destination.display()
            ));
        }
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
#[path = "../../tests/workflows/chunked_transform_internal_test.rs"]
mod internal_tests;

#[cfg(test)]
#[path = "../../tests/workflows/chunked_transform_segmented_evidence_test.rs"]
mod segmented_evidence_tests;

#[cfg(test)]
#[path = "../../tests/workflows/chunked_transform_openamip_structure_test.rs"]
mod openamip_structure_tests;
