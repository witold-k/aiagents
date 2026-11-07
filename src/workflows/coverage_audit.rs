// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Read-only, source-grounded replay count audit. Counts are diagnostics, not
//! a claim of semantic correctness or completeness of the published JSON.

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(super) struct CoverageAudit {
    schema_version: u32,
    candidate_facts: usize,
    distinct_statement_evidence_pairs: usize,
    explicitly_typed_candidates: usize,
    unresolved_candidates: usize,
    unresolved_candidate_facts: usize,
    statement_located: usize,
    statement_missing: usize,
    statement_exact_match: usize,
    statement_mismatch: usize,
    verified_parameter_assertions: usize,
    parameter_matched: usize,
    parameter_unmatched: usize,
    parameter_other: usize,
    checks: Vec<CoverageCheck>,
    caveats: Vec<&'static str>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CoverageCheck {
    name: &'static str,
    passed: bool,
}

impl CoverageAudit {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        candidate_facts: usize,
        distinct_pairs: usize,
        typed: usize,
        unresolved_candidates: usize,
        unresolved_facts: usize,
        locator_total: usize,
        located: usize,
        missing: usize,
        exact: usize,
        mismatch: usize,
        parameter_inventory: usize,
        parameter_unresolved: usize,
        matched: usize,
        unmatched: usize,
        other: usize,
    ) -> Self {
        let checks = vec![
            CoverageCheck {
                name: "distinct_pairs_not_greater_than_candidates",
                passed: distinct_pairs <= candidate_facts,
            },
            CoverageCheck {
                name: "locator_total_matches_candidates",
                passed: locator_total == candidate_facts,
            },
            CoverageCheck {
                name: "locator_partition",
                passed: located.checked_add(missing) == Some(locator_total),
            },
            CoverageCheck {
                name: "exact_within_located",
                passed: exact <= located,
            },
            CoverageCheck {
                name: "mismatch_within_located",
                passed: mismatch <= located,
            },
            CoverageCheck {
                name: "parameter_partition",
                passed: matched
                    .checked_add(unmatched)
                    .and_then(|n| n.checked_add(other))
                    == Some(parameter_unresolved),
            },
            CoverageCheck {
                name: "parameter_inventory_matches_coverage",
                passed: parameter_inventory == matched,
            },
            CoverageCheck {
                name: "unresolved_facts_within_candidates",
                passed: unresolved_facts <= candidate_facts,
            },
        ];
        Self {
            schema_version: 1,
            candidate_facts,
            distinct_statement_evidence_pairs: distinct_pairs,
            explicitly_typed_candidates: typed,
            unresolved_candidates,
            unresolved_candidate_facts: unresolved_facts,
            statement_located: located,
            statement_missing: missing,
            statement_exact_match: exact,
            statement_mismatch: mismatch,
            verified_parameter_assertions: parameter_inventory,
            parameter_matched: matched,
            parameter_unmatched: unmatched,
            parameter_other: other,
            checks,
            caveats: vec![
                "A missing statement locator is not proof of missing source evidence.",
                "Exact source location does not establish semantic correctness.",
                "This audit does not compare staged facts with published JSON.",
                "Unresolved identity does not imply an extraction error.",
            ],
        }
    }

    pub(super) fn passed(&self) -> bool {
        self.checks.iter().all(|check| check.passed)
    }
}

#[derive(Debug, Serialize)]
pub(super) struct MissingLocatorDetail {
    candidate_id: String,
    proposed_name: String,
    fact_index: usize,
    segment_id: String,
    statement: String,
    evidence_quote: Option<String>,
    source_window: Option<usize>,
    source_byte_offset: Option<usize>,
    source_locator_ambiguous: bool,
    exact_statement_occurrences_in_window: Option<usize>,
    exact_quote_occurrences_in_window: Option<usize>,
    reason: &'static str,
    unique_quote_byte_offset_in_window: Option<usize>,
    quote_at_source_byte_offset: Option<bool>,
}

/// Enumerate missing statement locators without inferring a replacement position.
/// A statement can be a valid paraphrase even when it does not occur verbatim.
pub(super) fn missing_locator_details(
    candidates: &[super::message_identity::MessageCandidate],
    windows: &[&str],
) -> Vec<MissingLocatorDetail> {
    let mut result = Vec::new();
    for candidate in candidates {
        let window_index = super::message_identity::source_window_index(&candidate.id);
        let window = window_index.and_then(|index| windows.get(index)).copied();
        for (fact_index, fact) in candidate.facts.iter().enumerate() {
            if fact.statement_byte_offset.is_some() {
                continue;
            }
            let statement_hits = window.map(|source| {
                if fact.statement.is_empty() {
                    0
                } else {
                    source.matches(&fact.statement).count()
                }
            });
            let quote_hits = window.and_then(|source| {
                fact.evidence_quote.as_ref().map(|quote| {
                    if quote.is_empty() {
                        0
                    } else {
                        source.matches(quote).count()
                    }
                })
            });
            // A unique quote is a source anchor, not proof that the statement is true.
            let unique_quote_offset = window.and_then(|source| {
                let quote = fact.evidence_quote.as_deref()?;
                if quote.is_empty() {
                    return None;
                }
                let mut matches = source.match_indices(quote);
                let first = matches.next()?.0;
                matches.next().is_none().then_some(first)
            });
            let quote_at_source_offset = window.and_then(|source| {
                let offset = fact.source_byte_offset?;
                let quote = fact.evidence_quote.as_deref()?;
                let end = offset.checked_add(quote.len())?;
                Some(source.get(offset..end) == Some(quote))
            });
            let reason = if window.is_none() {
                "source_window_unavailable"
            } else if fact.source_locator_ambiguous {
                "ambiguous_source_locator"
            } else if fact.statement.is_empty() {
                "empty_statement"
            } else {
                match statement_hits {
                    Some(0) => "statement_not_verbatim_in_window",
                    Some(1) => "unique_statement_without_locator",
                    Some(_) => "repeated_statement_in_window",
                    None => "source_window_unavailable",
                }
            };
            result.push(MissingLocatorDetail {
                candidate_id: candidate.id.clone(),
                proposed_name: candidate.proposed_name.clone(),
                fact_index,
                segment_id: fact.segment_id.clone(),
                statement: fact.statement.clone(),
                evidence_quote: fact.evidence_quote.clone(),
                source_window: window_index,
                source_byte_offset: fact.source_byte_offset,
                source_locator_ambiguous: fact.source_locator_ambiguous,
                exact_statement_occurrences_in_window: statement_hits,
                exact_quote_occurrences_in_window: quote_hits,
                reason,
                unique_quote_byte_offset_in_window: unique_quote_offset,
                quote_at_source_byte_offset: quote_at_source_offset,
            });
        }
    }
    result
}

#[derive(Debug, Serialize)]
pub(super) struct EvidenceCoverageReport {
    schema_version: u32,
    windows: usize,
    source_lines_considered: usize,
    source_lines_with_evidence: usize,
    source_lines_without_evidence: usize,
    source_lines_skipped: usize,
    unanchored_facts: usize,
    uncovered_lines_omitted: usize,
    uncovered_lines: Vec<UncoveredSourceLine>,
    caveats: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct UncoveredSourceLine {
    window_index: usize,
    line_number: usize,
    byte_offset_in_window: usize,
    text: String,
}

/// A deterministic evidence-footprint audit, not a semantic completeness test.
/// It highlights source lines without any overlapping, exact candidate evidence.
/// Lines with no quote are not necessarily missing facts; lines with a quote
/// are not necessarily correctly extracted. No parser or model call is added.
pub(super) fn evidence_coverage_report(
    candidates: &[super::message_identity::MessageCandidate],
    windows: &[&str],
) -> EvidenceCoverageReport {
    use std::collections::BTreeMap;
    const MAX_EXAMPLES: usize = 10_000;
    const MAX_LINE_CHARS: usize = 240;
    let mut anchors: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    let mut unanchored_facts = 0;
    for candidate in candidates {
        let Some(index) = super::message_identity::source_window_index(&candidate.id) else {
            unanchored_facts += candidate.facts.len();
            continue;
        };
        let Some(window) = windows.get(index) else {
            unanchored_facts += candidate.facts.len();
            continue;
        };
        for fact in &candidate.facts {
            let Some(quote) = fact.evidence_quote.as_deref().filter(|q| !q.is_empty()) else {
                unanchored_facts += 1;
                continue;
            };
            // Source offsets are byte offsets relative to the source window.
            // Require a byte-for-byte match, including UTF-8 boundaries.
            let verified = fact.source_byte_offset.and_then(|start| {
                let end = start.checked_add(quote.len())?;
                (window.get(start..end) == Some(quote)).then_some((start, end))
            });
            let anchor = verified.or_else(|| {
                let mut hits = window.match_indices(quote);
                let start = hits.next()?.0;
                hits.next()
                    .is_none()
                    .then_some((start, start + quote.len()))
            });
            if let Some(span) = anchor {
                anchors.entry(index).or_default().push(span);
            } else {
                unanchored_facts += 1;
            }
        }
    }
    let mut report = EvidenceCoverageReport {
        schema_version: 1,
        windows: windows.len(),
        source_lines_considered: 0,
        source_lines_with_evidence: 0,
        source_lines_without_evidence: 0,
        source_lines_skipped: 0,
        unanchored_facts,
        uncovered_lines_omitted: 0,
        uncovered_lines: Vec::new(),
        caveats: vec![
            "Evidence overlap is not semantic entailment or correctness.",
            "An uncovered line is a review candidate, not proof of an omitted fact.",
            "A covered line can contain additional unextracted facts.",
            "Whitespace-only lines, Markdown image payloads, and long encoded lines are excluded.",
            "The uncovered line inventory retains up to 10000 priority-ranked lines; totals are not capped.",
        ],
    };
    // Bounded, priority-ranked inventory across every source window.
    // Prefer technical evidence, preserving source order for equal priorities.
    let mut retained: BTreeMap<(u8, std::cmp::Reverse<usize>), UncoveredSourceLine> =
        BTreeMap::new();
    let mut sequence = 0usize;
    for (window_index, window) in windows.iter().enumerate() {
        let spans = anchors.get(&window_index).map(Vec::as_slice).unwrap_or(&[]);
        let mut offset = 0;
        for (line_index, raw_line) in window.split_inclusive('\n').enumerate() {
            let line = raw_line.trim();
            // Exclude embedded image/base64 payloads and other non-prose data.
            if line.is_empty() || line.starts_with("![](data:") || line.len() > 2000 {
                report.source_lines_skipped += 1;
            } else {
                report.source_lines_considered += 1;
                let end = offset + raw_line.len();
                if spans
                    .iter()
                    .any(|&(start, stop)| start < end && stop > offset)
                {
                    report.source_lines_with_evidence += 1;
                } else {
                    report.source_lines_without_evidence += 1;
                    let key = (technical_priority(line), std::cmp::Reverse(sequence));
                    sequence += 1;
                    if retained.len() < MAX_EXAMPLES
                        || retained
                            .first_key_value()
                            .is_some_and(|(lowest, _)| key > *lowest)
                    {
                        retained.insert(
                            key,
                            UncoveredSourceLine {
                                window_index,
                                line_number: line_index + 1,
                                byte_offset_in_window: offset,
                                text: line.chars().take(MAX_LINE_CHARS).collect(),
                            },
                        );
                        if retained.len() > MAX_EXAMPLES {
                            retained.pop_first();
                        }
                    }
                }
            }
            offset += raw_line.len();
        }
    }
    report.uncovered_lines_omitted = report
        .source_lines_without_evidence
        .saturating_sub(retained.len());
    report.uncovered_lines = retained.into_values().collect();
    report
        .uncovered_lines
        .sort_by_key(|line| (line.window_index, line.line_number));
    report
}

/// Global, source-line-based coverage. Overlapping windows are merged before
/// counting, so an anchor in any window covers the corresponding source line.
pub(super) fn evidence_coverage_report_global(
    candidates: &[super::message_identity::MessageCandidate],
    windows: &[&str],
    starts: &[usize],
    source: &str,
) -> EvidenceCoverageReport {
    use std::collections::BTreeMap;
    const MAX_EXAMPLES: usize = 10_000;
    const MAX_LINE_CHARS: usize = 240;
    let mut report = evidence_coverage_report(candidates, windows);
    let mut anchored = Vec::new();
    for candidate in candidates {
        let Some(index) = super::message_identity::source_window_index(&candidate.id) else {
            continue;
        };
        let Some((&window, &base)) = windows.get(index).zip(starts.get(index)) else {
            continue;
        };
        for fact in &candidate.facts {
            let Some(quote) = fact.evidence_quote.as_deref().filter(|q| !q.is_empty()) else {
                continue;
            };
            let span = fact
                .source_byte_offset
                .and_then(|start| {
                    let end = start.checked_add(quote.len())?;
                    (window.get(start..end) == Some(quote)).then_some((start, end))
                })
                .or_else(|| {
                    let mut hits = window.match_indices(quote);
                    let start = hits.next()?.0;
                    hits.next()
                        .is_none()
                        .then_some((start, start + quote.len()))
                });
            if let Some((a, b)) = span
                && let (Some(a), Some(b)) = (base.checked_add(a), base.checked_add(b))
                && source.get(a..b) == Some(quote)
            {
                anchored.push((a, b));
            }
        }
    }
    // One representative per physical line, regardless of window overlap.
    let mut lines: BTreeMap<usize, (usize, usize, usize, String)> = BTreeMap::new();
    for (index, window) in windows.iter().enumerate() {
        let Some(&base) = starts.get(index) else {
            continue;
        };
        let mut offset = 0usize;
        for (number, raw) in window.split_inclusive('\n').enumerate() {
            let Some(absolute) = base.checked_add(offset) else {
                break;
            };
            let end = absolute.saturating_add(raw.len());
            // A partial line is not a trustworthy source-line representative.
            if source.get(absolute..end) == Some(raw)
                && (absolute == 0 || source.as_bytes().get(absolute - 1) == Some(&b'\n'))
                && (end == source.len() || raw.ends_with('\n'))
            {
                lines
                    .entry(absolute)
                    .or_insert_with(|| (index, number + 1, offset, raw.trim().to_owned()));
            }
            offset += raw.len();
        }
    }
    report.source_lines_considered = 0;
    report.source_lines_with_evidence = 0;
    report.source_lines_without_evidence = 0;
    report.source_lines_skipped = 0;
    let mut retained: BTreeMap<(u8, std::cmp::Reverse<usize>), UncoveredSourceLine> =
        BTreeMap::new();
    for (sequence, (absolute, (window_index, line_number, offset, line))) in
        lines.into_iter().enumerate()
    {
        if line.is_empty() || line.starts_with("![](data:") || line.len() > 2000 {
            report.source_lines_skipped += 1;
            continue;
        }
        report.source_lines_considered += 1;
        let end = absolute
            + source[absolute..]
                .find('\n')
                .map_or(source.len() - absolute, |n| n + 1);
        if anchored.iter().any(|&(a, b)| a < end && b > absolute) {
            report.source_lines_with_evidence += 1;
            continue;
        }
        report.source_lines_without_evidence += 1;
        let key = (technical_priority(&line), std::cmp::Reverse(sequence));
        if retained.len() < MAX_EXAMPLES
            || retained
                .first_key_value()
                .is_some_and(|(lowest, _)| key > *lowest)
        {
            retained.insert(
                key,
                UncoveredSourceLine {
                    window_index,
                    line_number,
                    byte_offset_in_window: offset,
                    text: line.chars().take(MAX_LINE_CHARS).collect(),
                },
            );
            if retained.len() > MAX_EXAMPLES {
                retained.pop_first();
            }
        }
    }
    report.uncovered_lines_omitted = report
        .source_lines_without_evidence
        .saturating_sub(retained.len());
    report.uncovered_lines = retained.into_values().collect();
    report
        .uncovered_lines
        .sort_by_key(|line| (line.window_index, line.line_number));
    report.caveats.push("Global coverage counts distinct complete source lines represented by windows; partial boundary lines are excluded.");
    report
}

/// Deterministic review queue, not a semantic verdict.
#[derive(Debug, Serialize)]
pub(super) struct SemanticReviewPlan {
    schema_version: u32,
    fact_reviews_total: usize,
    fact_reviews_included: usize,
    source_reviews_total: usize,
    source_reviews_inventory_omitted: usize,
    source_reviews_included: usize,
    source_reviews_skipped_as_nontechnical: usize,
    source_reviews_with_cross_window_evidence: usize,
    fact_reviews: Vec<FactReviewTask>,
    source_reviews: Vec<SourceReviewTask>,
    caveats: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct FactReviewTask {
    candidate_id: String,
    fact_index: usize,
    statement: String,
    evidence_quote: Option<String>,
    source_context: Option<String>,
    evidence_verified_at_offset: bool,
    verdict: &'static str,
}

#[derive(Debug, Serialize)]
struct SourceReviewTask {
    window_index: usize,
    line_number: usize,
    source_excerpt: String,
    priority: &'static str,
    cross_window_evidence: bool,
    candidate_statements: Vec<String>,
    verdict: &'static str,
}

fn technical_priority(line: &str) -> u8 {
    let t = line.trim();
    if t.is_empty()
        || t.starts_with('#')
        || t.starts_with("---")
        || t.starts_with("Copyright")
        || t.starts_with("©")
        || t.starts_with("Table of Contents")
        || t.starts_with("www.")
        || t.starts_with("http://")
        || t.starts_with("https://")
    {
        return 0;
    }
    let lower = t.to_ascii_lowercase();
    // Short protocol assignments carry information even without prose verbs.
    // Do not mistake a table caption or column heading for a message fact.
    if lower.starts_with("table ")
        || lower.starts_with("figure ")
        || lower.starts_with("appendix ")
        || lower.starts_with("revision ")
    {
        return 0;
    }
    if t.split_whitespace().count() <= 4
        && t.split_whitespace().any(|token| {
            let token = token.trim_matches(|c: char| c == '|' || c == '`' || c == ',' || c == ';');
            let Some((name, value)) = token.split_once('=') else {
                return false;
            };
            !name.is_empty()
                && !value.is_empty()
                && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    {
        return 2;
    }
    let technical = [
        " must ",
        " shall ",
        " should ",
        " required",
        " optional",
        " command",
        " parameter",
        " message",
        " interval",
        " value",
        " seconds",
        " degrees",
        " db",
        " end=",
        " type:",
        " format",
        " transceiver",
        " antenna",
        " modem",
        " supported",
        " report",
        " if ",
        " when ",
        " 0 ",
        " 1 ",
    ];
    if technical.iter().any(|word| lower.contains(word))
        || (t.starts_with('|') && t.matches('|').count() >= 3)
    {
        2
    } else if t.len() >= 35 && t.split_whitespace().count() >= 6 {
        1
    } else {
        0
    }
}

// The review queue is a diagnostic view, not a second document parser.
// Only exclude unmistakable navigation and document-administration sections.
fn review_section_is_navigation(heading: &str) -> bool {
    let heading = heading
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_ascii_lowercase();
    heading.contains("table of contents")
        || heading.contains("revision history")
        || heading.contains("revision record")
        || heading.contains("list of figures")
        || heading.contains("list of tables")
}

fn review_line_is_navigation(line: &str) -> bool {
    let text = line.trim();
    let lower = text.to_ascii_lowercase();
    text.is_empty()
        || text.starts_with('#')
        || text.starts_with("---")
        || text.starts_with("Copyright")
        || text.starts_with('©')
        || text.starts_with("![](")
        || text.starts_with("![")
        || lower.starts_with("table of contents")
        || lower.starts_with("revision history")
        || lower.starts_with("revision record")
        || lower.starts_with("www.")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || (text.starts_with('|')
            && text
                .trim_matches('|')
                .trim()
                .chars()
                .all(|c| c == '-' || c == ':' || c == ' ' || c == '+'))
}

fn overlap_score(a: &str, b: &str) -> usize {
    let tokens: std::collections::BTreeSet<_> = a
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_ascii_lowercase())
        .collect();
    b.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_ascii_lowercase())
        .filter(|t| tokens.contains(t))
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

/// Build a prioritized, cross-window review queue. Lexical candidate ranking
/// helps a verifier find facts; it never establishes semantic equivalence.
pub(super) fn semantic_review_plan(
    candidates: &[super::message_identity::MessageCandidate],
    windows: &[&str],
    starts: &[usize],
    source: &str,
    coverage: &EvidenceCoverageReport,
) -> SemanticReviewPlan {
    const MAX_FACTS: usize = 256;
    const MAX_SOURCE: usize = 200;
    const CONTEXT_RADIUS: usize = 320;
    const MAX_MATCHES: usize = 8;
    let mut fact_reviews = Vec::new();
    let mut fact_reviews_total = 0;
    let all_facts: Vec<_> = candidates.iter().flat_map(|c| c.facts.iter()).collect();
    let mut evidence_quotes = Vec::new();
    for candidate in candidates {
        let index = super::message_identity::source_window_index(&candidate.id);
        for (fact_index, fact) in candidate.facts.iter().enumerate() {
            if let Some(quote) = fact.evidence_quote.as_deref().filter(|q| !q.is_empty())
                && let Some(source) = index.and_then(|i| windows.get(i)).copied()
            {
                let anchored = fact
                    .source_byte_offset
                    .and_then(|start| {
                        let end = start.checked_add(quote.len())?;
                        (source.get(start..end) == Some(quote)).then_some(())
                    })
                    .is_some();
                if anchored {
                    evidence_quotes.push(quote);
                }
            }
            fact_reviews_total += 1;
            if fact_reviews.len() >= MAX_FACTS {
                continue;
            }
            let source = index.and_then(|i| windows.get(i)).copied();
            let quote = fact.evidence_quote.as_deref().filter(|q| !q.is_empty());
            let verified = source.zip(quote).and_then(|(source, quote)| {
                let start = fact.source_byte_offset?;
                let end = start.checked_add(quote.len())?;
                (source.get(start..end) == Some(quote)).then_some((start, end))
            });
            let source_context = source.and_then(|source| {
                let (start, end) = verified.or_else(|| {
                    let quote = quote?;
                    let mut hits = source.match_indices(quote);
                    let start = hits.next()?.0;
                    hits.next()
                        .is_none()
                        .then_some((start, start + quote.len()))
                })?;
                let mut left = start.saturating_sub(CONTEXT_RADIUS);
                while !source.is_char_boundary(left) {
                    left += 1;
                }
                let mut right = end.saturating_add(CONTEXT_RADIUS).min(source.len());
                while !source.is_char_boundary(right) {
                    right -= 1;
                }
                source.get(left..right).map(str::to_owned)
            });
            fact_reviews.push(FactReviewTask {
                candidate_id: candidate.id.clone(),
                fact_index,
                statement: fact.statement.clone(),
                evidence_quote: fact.evidence_quote.clone(),
                source_context,
                evidence_verified_at_offset: verified.is_some(),
                verdict: "pending_semantic_review",
            });
        }
    }
    // Do not let document front matter consume the bounded review budget.
    // Compare against verified evidence from ALL windows, not just the
    // originating window. The comparison is conservative and byte-exact.
    // Map window-local byte offsets to physical lines in the complete source.
    // Window boundaries must not reset section ownership.
    let tree = super::source_tree::SourceTree::parse(source);
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(source.match_indices('\n').map(|(offset, _)| offset + 1))
        .collect();
    let location = |window: usize, offset: usize| -> Option<usize> {
        let local = windows.get(window)?;
        let absolute = starts.get(window)?.checked_add(offset)?;
        // Reject stale/mismatched window offsets rather than attributing a
        // review item to an unrelated line in the normalized document.
        if !local.is_char_boundary(offset)
            || offset >= local.len()
            || source
                .as_bytes()
                .get(absolute..absolute.checked_add(local.len() - offset)?)
                != Some(&local.as_bytes()[offset..])
        {
            return None;
        }
        Some(line_starts.partition_point(|&p| p <= absolute) - 1)
    };
    let navigation_sections: std::collections::BTreeSet<usize> = tree
        .sections
        .iter()
        .enumerate()
        .filter_map(|(index, section)| {
            let heading = source.lines().nth(section.heading_line)?;
            review_section_is_navigation(heading).then_some(index)
        })
        .collect();
    let eligible = |line: &UncoveredSourceLine| -> bool {
        let Some(global_line) = location(line.window_index, line.byte_offset_in_window) else {
            return false;
        };
        if review_line_is_navigation(&line.text) {
            return false;
        }
        let mut section = tree.section_at_line.get(global_line).copied().flatten();
        while let Some(index) = section {
            if navigation_sections.contains(&index) {
                return false;
            }
            section = tree.sections[index].parent;
        }
        !matches!(
            tree.block_at_line(global_line).map(|block| block.kind),
            Some(super::source_tree::BlockKind::FencedCode)
        )
    };
    let table_metadata = |line: &UncoveredSourceLine| -> bool {
        let Some(global_line) = location(line.window_index, line.byte_offset_in_window) else {
            return true;
        };
        let Some(super::source_tree::BlockKind::Table(index)) =
            tree.block_at_line(global_line).map(|block| block.kind)
        else {
            return false;
        };
        let table = &tree.tables[index];
        // A header or alignment rule describes columns, not a protocol assertion.
        global_line <= table.start_line + 1
            || line
                .text
                .trim()
                .trim_matches('|')
                .trim()
                .chars()
                .all(|c| c == '-' || c == ':' || c == ' ' || c == '+')
    };
    let mut ranked = coverage
        .uncovered_lines
        .iter()
        .filter_map(|line| {
            if !eligible(line) || table_metadata(line) {
                return None;
            }
            let priority = technical_priority(&line.text);
            if priority == 0 {
                return None;
            }
            let cross_window = evidence_quotes
                .iter()
                .any(|quote| quote.contains(&line.text) || line.text.contains(quote));
            Some((line, priority, cross_window))
        })
        .collect::<Vec<_>>();
    let source_reviews_skipped_as_nontechnical =
        coverage.uncovered_lines.len().saturating_sub(ranked.len());
    let source_reviews_with_cross_window_evidence = ranked.iter().filter(|(_, _, c)| *c).count();
    ranked.sort_by_key(|(line, priority, cross_window)| {
        (
            std::cmp::Reverse(*priority),
            *cross_window,
            line.window_index,
            line.line_number,
        )
    });
    // Reserve the first pass for section diversity. Once every eligible
    // section has a representative, fill remaining slots by normal priority.
    // Both passes preserve the existing deterministic ranking.
    let mut represented_sections = std::collections::BTreeSet::new();
    let mut diverse = Vec::new();
    let mut remainder = Vec::new();
    for item @ (line, _, _) in ranked {
        let section = location(line.window_index, line.byte_offset_in_window)
            .and_then(|global_line| tree.section_at_line.get(global_line).copied().flatten());
        if represented_sections.insert(section) {
            diverse.push(item);
        } else {
            remainder.push(item);
        }
    }
    diverse.extend(remainder);
    // Join neighboring uncovered source lines into a bounded review unit.
    // This preserves conditions and values that continue onto the next line.
    let mut selected = std::collections::BTreeSet::new();
    let mut selected_source_lines = std::collections::BTreeSet::new();
    let mut source_reviews = Vec::new();
    for (line, priority, cross_window) in diverse {
        if source_reviews.len() >= MAX_SOURCE {
            break;
        }
        let key = (line.window_index, line.line_number);
        if selected.contains(&key) {
            continue;
        }
        let Some(global_line) = location(line.window_index, line.byte_offset_in_window) else {
            continue;
        };
        if !selected_source_lines.insert(global_line) {
            continue;
        }
        selected.insert(key);
        let mut excerpt = line.text.clone();
        let mut last_global = global_line;
        // Table rows are atomic; never join adjacent independent parameters.
        let is_table = matches!(
            tree.block_at_line(global_line).map(|b| b.kind),
            Some(super::source_tree::BlockKind::Table(_))
        );
        for _ in 0..(if is_table { 0 } else { 2 }) {
            let next_global = last_global + 1;
            let Some((neighbor, _, _)) = coverage.uncovered_lines.iter().find_map(|neighbor| {
                let neighbor_global =
                    location(neighbor.window_index, neighbor.byte_offset_in_window)?;
                if neighbor_global != next_global {
                    return None;
                }
                let allowed = eligible(neighbor)
                    && !table_metadata(neighbor)
                    && tree.section_at_line.get(global_line)
                        == tree.section_at_line.get(neighbor_global)
                    && tree.blocks.iter().position(|block| {
                        block.start_line <= global_line && global_line < block.end_line
                    }) == tree.blocks.iter().position(|block| {
                        block.start_line <= neighbor_global && neighbor_global < block.end_line
                    });
                allowed.then_some((neighbor, technical_priority(&neighbor.text), false))
            }) else {
                break;
            };
            if selected.contains(&(neighbor.window_index, neighbor.line_number))
                || review_line_is_navigation(&neighbor.text)
            {
                break;
            }
            match location(neighbor.window_index, neighbor.byte_offset_in_window) {
                Some(global_neighbor) if !selected_source_lines.insert(global_neighbor) => break,
                _ => {}
            }
            excerpt.push('\n');
            excerpt.push_str(&neighbor.text);
            selected.insert((neighbor.window_index, neighbor.line_number));
            last_global = next_global;
        }
        let mut matches = all_facts
            .iter()
            .map(|fact| {
                let score = overlap_score(&excerpt, &fact.statement).max(
                    fact.evidence_quote
                        .as_deref()
                        .map(|q| overlap_score(&excerpt, q))
                        .unwrap_or(0),
                );
                (score, fact.statement.as_str())
            })
            .filter(|(score, _)| *score > 0)
            .collect::<Vec<_>>();
        matches.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        let mut candidate_statements = Vec::new();
        for (_, statement) in matches {
            if !candidate_statements
                .iter()
                .any(|existing| existing == statement)
            {
                candidate_statements.push(statement.to_owned());
            }
            if candidate_statements.len() >= MAX_MATCHES {
                break;
            }
        }
        source_reviews.push(SourceReviewTask {
            window_index: line.window_index,
            line_number: line.line_number,
            source_excerpt: excerpt,
            priority: if priority == 2 { "technical" } else { "prose" },
            cross_window_evidence: cross_window,
            candidate_statements,
            verdict: "pending_missing_fact_review",
        });
    }
    SemanticReviewPlan {
        schema_version: 4,
        fact_reviews_total,
        fact_reviews_included: fact_reviews.len(),
        source_reviews_total: coverage.source_lines_without_evidence,
        source_reviews_inventory_omitted: coverage.uncovered_lines_omitted,
        source_reviews_included: source_reviews.len(),
        source_reviews_skipped_as_nontechnical,
        source_reviews_with_cross_window_evidence,
        fact_reviews,
        source_reviews,
        caveats: vec![
            "Review queue only: no LLM semantic verification has been performed.",
            "Cross-window evidence means byte-exact quote/line containment, not semantic coverage.",
            "Candidate statements are globally ranked by token overlap, not semantic similarity.",
            "Uncovered source lines are not proof of missing facts.",
            "Review coordinates use the normalized document shared with source windows; original offsets are a separate coordinate space.",
            "Navigation sections use full-document source-tree ancestry; table rows are atomic and prose joins only within one source block.",
            "Overlapping windows do not create duplicate review units for the same normalized source line.",
            "Priority is a heuristic and can miss important source statements.",
            "Uncovered line inventory is priority-ranked and capped at 10000; review queue at 200 source units.",
        ],
    }
}

#[derive(Debug, serde::Serialize)]
pub(super) struct SemanticVerificationReport {
    schema_version: u32,
    attempted: usize,
    accepted: usize,
    rejected: usize,
    reviews: Vec<SemanticVerdict>,
}

#[derive(Debug, serde::Serialize)]
struct SemanticVerdict {
    window_index: usize,
    line_number: usize,
    source_excerpt: String,
    verdict: String,
    matched_candidate_indices: Vec<usize>,
    rationale: String,
}

impl SemanticReviewPlan {
    pub(super) fn verifier_tasks(&self, limit: usize) -> Vec<(usize, usize, String, Vec<String>)> {
        self.source_reviews
            .iter()
            .take(limit)
            .map(|task| {
                (
                    task.window_index,
                    task.line_number,
                    task.source_excerpt.clone(),
                    task.candidate_statements.clone(),
                )
            })
            .collect()
    }
}

impl SemanticVerificationReport {
    pub(super) fn new() -> Self {
        Self {
            schema_version: 1,
            attempted: 0,
            accepted: 0,
            rejected: 0,
            reviews: Vec::new(),
        }
    }

    pub(super) fn record(
        &mut self,
        window_index: usize,
        line_number: usize,
        excerpt: &str,
        candidate_count: usize,
        response: &str,
    ) {
        self.attempted += 1;
        let cleaned = response.trim().trim_matches('`').trim();
        let cleaned = cleaned.strip_prefix("json").unwrap_or(cleaned).trim();
        let valid = serde_json::from_str::<serde_json::Value>(cleaned)
            .ok()
            .and_then(|value| {
                let verdict = value.get("verdict")?.as_str()?;
                if !["covered", "partially_covered", "missing", "uncertain"].contains(&verdict) {
                    return None;
                }
                let rationale = value.get("rationale")?.as_str()?.trim();
                if rationale.is_empty() || rationale.len() > 2000 {
                    return None;
                }
                let indices = value.get("matched_candidate_indices")?.as_array()?;
                let mut matched = Vec::new();
                for index in indices {
                    let index = usize::try_from(index.as_u64()?).ok()?;
                    if index >= candidate_count || matched.contains(&index) {
                        return None;
                    }
                    matched.push(index);
                }
                if verdict == "missing" && !matched.is_empty() {
                    return None;
                }
                if verdict == "covered" && matched.is_empty() {
                    return None;
                }
                Some((verdict.to_owned(), matched, rationale.to_owned()))
            });
        if let Some((verdict, matched_candidate_indices, rationale)) = valid {
            self.accepted += 1;
            self.reviews.push(SemanticVerdict {
                window_index,
                line_number,
                source_excerpt: excerpt.to_owned(),
                verdict,
                matched_candidate_indices,
                rationale,
            });
        } else {
            self.rejected += 1;
            self.reviews.push(SemanticVerdict {
                window_index,
                line_number,
                source_excerpt: excerpt.to_owned(),
                verdict: "uncertain".to_owned(),
                matched_candidate_indices: Vec::new(),
                rationale: "Invalid LLM response; no coverage conclusion.".to_owned(),
            });
        }
    }
}

pub(super) const SEMANTIC_VERIFIER_PROMPT: &str = r#"Compare an OpenAMIP source excerpt to extracted statements. Judge full technical equivalence including case-sensitive command letters (V differs from v), sender/receiver, conditions, units, ranges and values. Ignore mere lexical similarity. Headings and metadata are uncertain, not missing. Return ONLY JSON with verdict (covered|partially_covered|missing|uncertain), matched_candidate_indices (zero-based integer array), rationale (short string). Covered requires a matching candidate; missing requires an empty array. Never invent facts."#;

#[cfg(test)]
#[path = "../../tests/workflows/coverage_audit_test.rs"]
mod tests;
