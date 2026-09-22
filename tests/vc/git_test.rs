// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::vc::git::Git;

#[test]
fn reads_repository_history() {
    let git = Git::new(env!("CARGO_MANIFEST_DIR"));

    assert!(git.is_repository().unwrap());
    assert_eq!(
        git.repository_root().unwrap(),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    );

    let commits = git.commits(None, Some("HEAD")).unwrap();
    assert!(!commits.is_empty());
    assert!(!commits[0].hash.is_empty());
    assert!(!commits[0].subject.is_empty());
}


#[test]
fn reads_diff_between_refs() {
    let git = Git::new(env!("CARGO_MANIFEST_DIR"));
    let diff = git.diff("HEAD~1", "HEAD").unwrap();

    assert!(!diff.is_empty());
}

#[test]
fn lists_and_reads_files_at_ref() {
    let git = Git::new(env!("CARGO_MANIFEST_DIR"));
    let files = git.files("HEAD").unwrap();

    assert!(files.iter().any(|path| path == std::path::Path::new("Cargo.toml")));

    let cargo_toml = git.file_at("HEAD", "Cargo.toml").unwrap();
    let cargo_toml = String::from_utf8(cargo_toml).unwrap();

    assert!(cargo_toml.contains("[package]"));
}
