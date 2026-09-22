// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::vc::git::Git;
use aiagents::vc::release_history::ReleaseHistory;

#[test]
fn builds_history_since_latest_tag() {
    let git = Git::new(env!("CARGO_MANIFEST_DIR"));
    let history = ReleaseHistory::since_latest_tag(&git).unwrap();

    assert_eq!(history.to, "HEAD");

    if let Some(tag) = history.from.as_deref() {
        let expected = git.commits(Some(tag), Some("HEAD")).unwrap();
        assert_eq!(history.commits, expected);
    } else {
        let expected = git.commits(None, Some("HEAD")).unwrap();
        assert_eq!(history.commits, expected);
    }
}

#[test]
fn builds_history_between_refs() {
    let git = Git::new(env!("CARGO_MANIFEST_DIR"));
    let history = ReleaseHistory::between(&git, "HEAD~1", "HEAD").unwrap();

    assert_eq!(history.from.as_deref(), Some("HEAD~1"));
    assert_eq!(history.to, "HEAD");
    assert_eq!(history.commits.len(), 1);
}
