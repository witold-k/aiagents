// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::git_fixture::TestRepository;

#[test]
fn reads_repository_history() {
    let repository = TestRepository::new();
    let git = repository.git();

    assert!(git.is_repository().unwrap());
    assert_eq!(git.repository_root().unwrap(), repository.path());

    let commits = git.commits(None, Some("HEAD")).unwrap();
    assert_eq!(commits.len(), 2);
    assert!(!commits[0].hash.is_empty());
    assert!(!commits[0].subject.is_empty());
}

#[test]
fn reads_diff_between_refs() {
    let repository = TestRepository::new();
    let git = repository.git();
    let diff = git.diff("HEAD~1", "HEAD").unwrap();

    assert!(!diff.is_empty());
    assert!(diff.contains("Cargo.toml"));
}

#[test]
fn lists_and_reads_files_at_ref() {
    let repository = TestRepository::new();
    let git = repository.git();
    let files = git.files("HEAD").unwrap();

    assert!(files.iter().any(|path| path == std::path::Path::new("Cargo.toml")));

    let cargo_toml = git.file_at("HEAD", "Cargo.toml").unwrap();
    let cargo_toml = String::from_utf8(cargo_toml).unwrap();

    assert!(cargo_toml.contains("[package]"));
}
