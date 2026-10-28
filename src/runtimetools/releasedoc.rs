// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::PathBuf;

use crate::vc::git::{Git, GitCommit, GitError};
use crate::vc::release_history::ReleaseHistory;

/// The net change of one file between release endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseFileChange {
    pub path: PathBuf,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDocContext {
    pub from: Option<String>,
    pub to: String,
    pub commits: Vec<GitCommit>,
    pub changes: Vec<ReleaseFileChange>,
}

impl ReleaseDocContext {
    pub fn since_latest_tag(git: &Git) -> Result<Self, GitError> {
        Self::from_history(git, ReleaseHistory::since_latest_tag(git)?)
    }

    pub fn between(git: &Git, from: &str, to: &str) -> Result<Self, GitError> {
        Self::from_history(git, ReleaseHistory::between(git, from, to)?)
    }

    fn from_history(git: &Git, history: ReleaseHistory) -> Result<Self, GitError> {
        // The empty tree represents the baseline for a repository's first release.
        const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
        let baseline = history.from.as_deref().unwrap_or(EMPTY_TREE);
        let changes = git
            .changed_file_diffs(baseline, &history.to)?
            .into_iter()
            .map(|(path, diff)| ReleaseFileChange { path, diff })
            .collect();
        Ok(Self {
            from: history.from,
            to: history.to,
            commits: history.commits,
            changes,
        })
    }

    pub fn release_metadata(&self) -> String {
        let mut context = format!(
            "=== RELEASE RANGE ===\nfrom: {}\nto: {}\n\n=== COMMITS ===\n",
            self.from.as_deref().unwrap_or("<repository start>"),
            self.to,
        );
        for commit in &self.commits {
            context.push_str(&format!(
                "hash: {}\ndate: {}\nsubject: {}\nbody:\n{}\n---\n",
                commit.hash, commit.authored_at, commit.subject, commit.body,
            ));
        }
        context
    }
}
