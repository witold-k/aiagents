// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::workflows::release_doc::release_doc_path_allowed;
use aiagents::config::Config;
use std::path::Path;

#[test]
fn release_doc_uses_both_scanner_filters() {
    let mut config = Config::default();
    config.scanendfilter = vec![".rs".into(), ".cpp".into()];
    config.scanfullfilter = vec!["Cargo.toml".into(), "Makefile".into()];
    assert!(release_doc_path_allowed(Path::new("src/main.rs"), &config));
    assert!(release_doc_path_allowed(
        Path::new("nested/Cargo.toml"),
        &config
    ));
    assert!(release_doc_path_allowed(Path::new("Makefile"), &config));
    assert!(!release_doc_path_allowed(
        Path::new("assets/logo.png"),
        &config
    ));
    assert!(!release_doc_path_allowed(
        Path::new("Cargo.toml.bak"),
        &config
    ));
    assert!(!release_doc_path_allowed(
        Path::new("src/main.rs.bak"),
        &config
    ));
    assert!(!release_doc_path_allowed(
        Path::new("nested/Makefile.old"),
        &config
    ));
}

#[test]
fn release_doc_empty_filters_exclude_everything() {
    let mut config = Config::default();
    config.scanendfilter.clear();
    config.scanfullfilter.clear();
    assert!(!release_doc_path_allowed(Path::new("src/main.rs"), &config));
    assert!(!release_doc_path_allowed(Path::new("Cargo.toml"), &config));
}
