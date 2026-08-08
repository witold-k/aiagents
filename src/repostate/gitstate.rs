// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::{ Path, PathBuf };
use std::process::Command;
use crate::repostate::RepoState;

#[derive(Clone)]
pub struct GitState {
    path: PathBuf
}

impl GitState {
    pub fn from_path(path: &Path) -> GitState {
        GitState { path: path.to_path_buf() }
    }
}

impl RepoState for GitState {
    /// does a `git add -A` on directory self.path
    fn commit(&self) -> bool {
        let status = Command::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(&self.path)
            .status();

        matches!(status, Ok(s) if s.success())
    }

    /// does a `git checkout .` on directory self.path
    fn restore(&self) -> bool {
        let status = Command::new("git")
            .arg("checkout")
            .arg(".")
            .current_dir(&self.path)
            .status();

        matches!(status, Ok(s) if s.success())
    }
}

