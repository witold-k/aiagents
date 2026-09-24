// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::PathBuf;

use crate::vc::git::{Git, GitCommit, GitError};
use crate::vc::release_history::ReleaseHistory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseFile {
    pub path: PathBuf,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDocContext {
    pub from: Option<String>,
    pub to: String,
    pub commits: Vec<GitCommit>,
    pub diff: Option<String>,
    pub files: Vec<ReleaseFile>,
}

impl ReleaseDocContext {
    pub fn since_latest_tag(git: &Git) -> Result<Self, GitError> {
        let history = ReleaseHistory::since_latest_tag(git)?;
        Self::from_history(git, history)
    }

    pub fn between(git: &Git, from: &str, to: &str) -> Result<Self, GitError> {
        let history = ReleaseHistory::between(git, from, to)?;
        Self::from_history(git, history)
    }

    fn from_history(git: &Git, history: ReleaseHistory) -> Result<Self, GitError> {
        let diff = history
            .from
            .as_deref()
            .map(|from| git.diff(from, &history.to))
            .transpose()?;

        let files = git
            .files(&history.to)?
            .into_iter()
            .map(|path| {
                let content = git.file_at(&history.to, &path)?;
                Ok(ReleaseFile { path, content })
            })
            .collect::<Result<Vec<_>, GitError>>()?;

        Ok(Self {
            from: history.from,
            to: history.to,
            commits: history.commits,
            diff,
            files,
        })
    }

    pub fn to_llm_context(&self) -> String {
        let mut context = String::new();

        context.push_str("=== RELEASE RANGE ===\n");
        context.push_str(&format!(
            "from: {}\nto: {}\n",
            self.from.as_deref().unwrap_or("<repository start>"),
            self.to
        ));

        context.push_str("\n=== COMMITS ===\n");
        for commit in &self.commits {
            context.push_str(&format!(
                "hash: {}\nauthor: {} <{}>\ndate: {}\nsubject: {}\nbody:\n{}\n---\n",
                commit.hash,
                commit.author_name,
                commit.author_email,
                commit.authored_at,
                commit.subject,
                commit.body
            ));
        }

        context.push_str("\n=== DIFF ===\n");
        match &self.diff {
            Some(diff) => context.push_str(diff),
            None => context.push_str("<no previous tag; no range diff available>"),
        }

        context.push_str("\n\n=== FILES AT TARGET REF ===\n");
        for file in &self.files {
            context.push_str(&format!("\n--- FILE: {} ---\n", file.path.display()));
            match std::str::from_utf8(&file.content) {
                Ok(content) => context.push_str(content),
                Err(_) => context.push_str("<binary file omitted>"),
            }
            context.push_str("\n--- END FILE ---\n");
        }

        context
    }
    pub fn to_llm_summary_context(&self) -> String {
        let mut context = String::new();

        context.push_str("=== RELEASE RANGE ===\n");
        context.push_str(&format!(
            "from: {}\nto: {}\n",
            self.from.as_deref().unwrap_or("<repository start>"),
            self.to
        ));

        context.push_str("\n=== COMMITS ===\n");
        for commit in &self.commits {
            context.push_str(&format!(
                "hash: {}\ndate: {}\nsubject: {}\nbody:\n{}\n---\n",
                commit.hash,
                commit.authored_at,
                commit.subject,
                commit.body
            ));
        }

        context.push_str("\n=== DIFF ===\n");
        match &self.diff {
            Some(diff) => context.push_str(diff),
            None => context.push_str("<no previous tag; no range diff available>"),
        }

        context.push_str("\n\n=== AVAILABLE FILES AT TARGET REF ===\n");
        for file in &self.files {
            context.push_str(&file.path.to_string_lossy());
            context.push('\n');
        }

        context
    }

}
