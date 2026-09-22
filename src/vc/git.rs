// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FIELD_SEPARATOR: char = '\x1f';
const RECORD_SEPARATOR: char = '\x1e';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommit {
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_at: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug)]
pub enum GitError {
    Io(std::io::Error),
    CommandFailed {
        args: Vec<String>,
        status: Option<i32>,
        stderr: String,
    },
    InvalidOutput(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::Io(err) => write!(f, "failed to execute git: {err}"),
            GitError::CommandFailed { args, status, stderr } => {
                write!(
                    f,
                    "git {} failed with status {:?}: {}",
                    args.join(" "),
                    status,
                    stderr.trim()
                )
            }
            GitError::InvalidOutput(message) => write!(f, "invalid git output: {message}"),
        }
    }
}

impl std::error::Error for GitError {}

impl From<std::io::Error> for GitError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone)]
pub struct Git {
    dir: PathBuf,
}

impl Git {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn is_repository(&self) -> Result<bool, GitError> {
        let output = self.run_raw(["rev-parse", "--is-inside-work-tree"])?;
        Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true")
    }

    pub fn repository_root(&self) -> Result<PathBuf, GitError> {
        let value = self.run(["rev-parse", "--show-toplevel"])?;
        Ok(PathBuf::from(value.trim()))
    }

    pub fn current_branch(&self) -> Result<Option<String>, GitError> {
        let branch = self.run(["branch", "--show-current"])?;
        let branch = branch.trim();
        if branch.is_empty() {
            Ok(None)
        } else {
            Ok(Some(branch.to_owned()))
        }
    }

    pub fn tags(&self) -> Result<Vec<String>, GitError> {
        let output = self.run(["tag", "--list", "--sort=version:refname"])?;
        Ok(output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }

    pub fn latest_tag(&self) -> Result<Option<String>, GitError> {
        let output = self.run_raw(["describe", "--tags", "--abbrev=0"])?;
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            Ok((!value.is_empty()).then_some(value))
        } else {
            Ok(None)
        }
    }

    pub fn commits(&self, from: Option<&str>, to: Option<&str>) -> Result<Vec<GitCommit>, GitError> {
        let range = match (from, to) {
            (Some(from), Some(to)) => Some(format!("{from}..{to}")),
            (Some(from), None) => Some(format!("{from}..HEAD")),
            (None, Some(to)) => Some(to.to_owned()),
            (None, None) => None,
        };

        let format = format!(
            "--format=%H{fs}%an{fs}%ae{fs}%aI{fs}%s{fs}%b{rs}",
            fs = FIELD_SEPARATOR,
            rs = RECORD_SEPARATOR,
        );

        let mut args = vec!["log".to_owned(), "--no-decorate".to_owned(), format];
        if let Some(range) = range {
            args.push(range);
        }

        let output = self.run(args.iter().map(String::as_str))?;
        parse_commits(&output)
    }

    pub fn diff(&self, from: &str, to: &str) -> Result<String, GitError> {
        self.run(["diff", "--no-ext-diff", "--binary", from, to])
    }

    pub fn files(&self, reference: &str) -> Result<Vec<PathBuf>, GitError> {
        let output = self.run(["ls-tree", "-r", "--name-only", reference])?;
        Ok(output
            .lines()
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect())
    }

    pub fn file_at(&self, reference: &str, path: impl AsRef<Path>) -> Result<Vec<u8>, GitError> {
        let object = format!("{reference}:{}", path.as_ref().to_string_lossy());
        let output = self.run_raw(["show", "--no-ext-diff", &object])?;

        if !output.status.success() {
            return Err(GitError::CommandFailed {
                args: vec!["show".to_owned(), "--no-ext-diff".to_owned(), object],
                status: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(output.stdout)
    }

    fn run<I, S>(&self, args: I) -> Result<String, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args_vec: Vec<String> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_string_lossy().into_owned())
            .collect();
        let output = self.run_raw(args_vec.iter().map(String::as_str))?;

        if !output.status.success() {
            return Err(GitError::CommandFailed {
                args: args_vec,
                status: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn run_raw<I, S>(&self, args: I) -> Result<Output, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Ok(Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .output()?)
    }
}

fn parse_commits(output: &str) -> Result<Vec<GitCommit>, GitError> {
    output
        .split(RECORD_SEPARATOR)
        .map(str::trim)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let mut fields = record.splitn(6, FIELD_SEPARATOR);
            let hash = fields.next();
            let author_name = fields.next();
            let author_email = fields.next();
            let authored_at = fields.next();
            let subject = fields.next();
            let body = fields.next();

            match (hash, author_name, author_email, authored_at, subject, body) {
                (
                    Some(hash),
                    Some(author_name),
                    Some(author_email),
                    Some(authored_at),
                    Some(subject),
                    Some(body),
                ) => Ok(GitCommit {
                    hash: hash.to_owned(),
                    author_name: author_name.to_owned(),
                    author_email: author_email.to_owned(),
                    authored_at: authored_at.to_owned(),
                    subject: subject.to_owned(),
                    body: body.trim_end().to_owned(),
                }),
                _ => Err(GitError::InvalidOutput(record.to_owned())),
            }
        })
        .collect()
}

