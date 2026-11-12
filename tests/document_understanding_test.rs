// SPDX-License-Identifier: Apache-2.0
use aiagents::document_understanding::{SemanticDocumentTree, SemanticNode, WindowBudget};

fn budget(chars: usize) -> WindowBudget {
    WindowBudget {
        context_tokens: chars + 10,
        output_tokens: 5,
        reserved_tokens: 5,
        prefix_tokens: 0,
        chars_per_token: 1,
    }
}

#[test]
fn preserves_source_and_switches_context_at_internal_boundary() {
    let source = "abcdef123456";
    let tree = SemanticDocumentTree {
        nodes: vec![
            SemanticNode {
                title: "First".into(),
                start_byte: 0,
                end_byte: 6,
                parent: None,
                context: vec!["Definition A".into()],
            },
            SemanticNode {
                title: "Second".into(),
                start_byte: 6,
                end_byte: 12,
                parent: None,
                context: vec!["Definition B".into()],
            },
        ],
    };
    let windows = tree.plan(source, budget(10)).unwrap();
    assert_eq!(
        windows
            .iter()
            .map(|w| &source[w.start_byte..w.end_byte])
            .collect::<String>(),
        source
    );
    assert_eq!(windows.len(), 2);
    assert!(windows[0].prefix.contains("Definition A"));
    assert!(!windows[0].prefix.contains("Definition B"));
    assert!(windows[1].prefix.contains("Definition B"));
}

#[test]
fn long_section_is_split_without_losing_context() {
    let source = "a".repeat(30);
    let tree = SemanticDocumentTree {
        nodes: vec![SemanticNode {
            title: "Root".into(),
            start_byte: 0,
            end_byte: 30,
            parent: None,
            context: vec![],
        }],
    };
    let windows = tree.plan(&source, budget(8)).unwrap();
    assert_eq!(windows.len(), 4);
    assert!(windows.iter().all(|w| w.prefix.contains("Root")));
    assert_eq!(
        windows
            .iter()
            .map(|w| w.end_byte - w.start_byte)
            .sum::<usize>(),
        30
    );
}

#[test]
fn nested_context_and_unicode_boundaries() {
    let source = "éééé";
    let tree = SemanticDocumentTree {
        nodes: vec![
            SemanticNode {
                title: "Root".into(),
                start_byte: 0,
                end_byte: 8,
                parent: None,
                context: vec!["general".into()],
            },
            SemanticNode {
                title: "Child".into(),
                start_byte: 4,
                end_byte: 8,
                parent: Some(0),
                context: vec!["specific".into()],
            },
        ],
    };
    let windows = tree.plan(source, budget(3)).unwrap();
    assert_eq!(
        windows
            .iter()
            .map(|w| &source[w.start_byte..w.end_byte])
            .collect::<String>(),
        source
    );
    assert!(windows.last().unwrap().prefix.contains("general"));
    assert!(windows.last().unwrap().prefix.contains("specific"));
}

#[test]
fn rejects_invalid_spans_and_exhausted_budget() {
    let source = "abc";
    let tree = SemanticDocumentTree {
        nodes: vec![SemanticNode {
            title: "Bad".into(),
            start_byte: 0,
            end_byte: 4,
            parent: None,
            context: vec![],
        }],
    };
    assert!(tree.plan(source, budget(10)).is_err());
    assert!(
        SemanticDocumentTree { nodes: vec![] }
            .plan(source, budget(0))
            .is_err()
    );
}
