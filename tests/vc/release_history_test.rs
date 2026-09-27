// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::vc::release_history::ReleaseHistory;

use crate::git_fixture::TestRepository;

#[test]
fn builds_history_since_latest_tag() {
    let repository = TestRepository::new();
    let git = repository.git();
    let history = ReleaseHistory::since_latest_tag(&git).unwrap();

    assert_eq!(history.from.as_deref(), Some("v0.1.0"));
    assert_eq!(history.to, "HEAD");
    assert_eq!(history.commits.len(), 1);
}

#[test]
fn builds_history_between_refs() {
    let repository = TestRepository::new();
    let git = repository.git();
    let history = ReleaseHistory::between(&git, "HEAD~1", "HEAD").unwrap();

    assert_eq!(history.from.as_deref(), Some("HEAD~1"));
    assert_eq!(history.to, "HEAD");
    assert_eq!(history.commits.len(), 1);
}
