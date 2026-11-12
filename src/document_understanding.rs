// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Experimental, source-grounded adaptive windows. No LLM hypotheses are
//! promoted to verified document structure by this planner.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticNode {
    pub title: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub parent: Option<usize>,
    /// Human-reviewed or deterministic source-backed context, never inferred ownership.
    pub context: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticDocumentTree {
    pub nodes: Vec<SemanticNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedWindow {
    pub start_byte: usize,
    pub end_byte: usize,
    pub prefix: String,
    pub active_node: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowBudget {
    pub context_tokens: usize,
    pub output_tokens: usize,
    pub reserved_tokens: usize,
    pub prefix_tokens: usize,
    /// Conservative input-token estimate; replace with the configured tokenizer.
    pub chars_per_token: usize,
}

impl WindowBudget {
    pub fn source_char_budget(self) -> usize {
        self.context_tokens
            .saturating_sub(self.output_tokens)
            .saturating_sub(self.reserved_tokens)
            .saturating_sub(self.prefix_tokens)
            .saturating_mul(self.chars_per_token.max(1))
    }
}

fn boundary(source: &str, position: usize) -> usize {
    let mut pos = position.min(source.len());
    while pos > 0 && !source.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

impl SemanticDocumentTree {
    pub fn validate(&self, source: &str) -> Result<(), String> {
        for (i, node) in self.nodes.iter().enumerate() {
            if node.start_byte >= node.end_byte || node.end_byte > source.len()
                || !source.is_char_boundary(node.start_byte)
                || !source.is_char_boundary(node.end_byte)
            {
                return Err(format!("invalid source span at node {i}"));
            }
            if let Some(parent) = node.parent {
                if parent >= i {
                    return Err(format!("parent must precede child at node {i}"));
                }
                let p = &self.nodes[parent];
                if node.start_byte < p.start_byte || node.end_byte > p.end_byte {
                    return Err(format!("child outside parent at node {i}"));
                }
            }
        }
        Ok(())
    }

    fn active_at(&self, offset: usize) -> Option<usize> {
        self.nodes.iter().enumerate()
            .filter(|(_, n)| n.start_byte <= offset && offset < n.end_byte)
            .min_by_key(|(_, n)| n.end_byte - n.start_byte)
            .map(|(i, _)| i)
    }

    pub fn prefix_at(&self, offset: usize) -> (Option<usize>, String) {
        let active = self.active_at(offset);
        let mut path = Vec::new();
        let mut cursor = active;
        while let Some(i) = cursor {
            path.push(i);
            cursor = self.nodes[i].parent;
        }
        path.reverse();
        let mut prefix = String::from("SOURCE-GROUNDED CONTEXT (not evidence):\n");
        for i in path {
            let node = &self.nodes[i];
            prefix.push_str(&format!("- {} [bytes {}..{}]\n", node.title, node.start_byte, node.end_byte));
            for fact in &node.context {
                prefix.push_str(&format!("  - {fact}\n"));
            }
        }
        (active, prefix)
    }

    /// Plan large windows, preferring semantic boundaries without skipping text.
    /// Each window gets context for its first byte; internal boundaries are
    /// split so the prefix cannot silently describe a different node.
    pub fn plan(&self, source: &str, budget: WindowBudget) -> Result<Vec<PlannedWindow>, String> {
        self.validate(source)?;
        let limit = budget.source_char_budget();
        if limit == 0 {
            return Err("source token budget is zero".into());
        }
        let mut result = Vec::new();
        let mut start = 0;
        while start < source.len() {
            let (active_node, prefix) = self.prefix_at(start);
            let mut end = start;
            for (chars, (offset, ch)) in source[start..].char_indices().enumerate() {
                if chars >= limit {
                    break;
                }
                end = start + offset + ch.len_utf8();
            }
            // Split at any semantic transition before the maximum end.
            for node in &self.nodes {
                for pos in [node.start_byte, node.end_byte] {
                    if pos > start && pos < end {
                        end = end.min(pos);
                    }
                }
            }
            end = boundary(source, end);
            if end <= start { return Err("planner made no progress".into()); }
            result.push(PlannedWindow { start_byte: start, end_byte: end, prefix, active_node });
            start = end;
        }
        Ok(result)
    }
}
