// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use super::git::{Git, GitCommit, GitError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseHistory {
    pub from: Option<String>,
    pub to: String,
    pub commits: Vec<GitCommit>,
}

impl ReleaseHistory {
    pub fn since_latest_tag(git: &Git) -> Result<Self, GitError> {
        let from = git.latest_tag()?;
        let commits = git.commits(from.as_deref(), Some("HEAD"))?;

        Ok(Self {
            from,
            to: "HEAD".to_owned(),
            commits,
        })
    }

    pub fn between(git: &Git, from: &str, to: &str) -> Result<Self, GitError> {
        let commits = git.commits(Some(from), Some(to))?;

        Ok(Self {
            from: Some(from.to_owned()),
            to: to.to_owned(),
            commits,
        })
    }
}
