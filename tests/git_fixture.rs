// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use aiagents::vc::git::Git;

pub struct TestRepository {
    path: PathBuf,
}

impl TestRepository {
    pub fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "aiagents-git-test-{}-{unique}",
            std::process::id()
        ));

        fs::create_dir(&path).unwrap();
        run_git(&path, ["init"]);
        run_git(&path, ["config", "user.name", "AI Agents Test"]);
        run_git(&path, ["config", "user.email", "tests@example.invalid"]);

        fs::write(path.join("README.md"), "first\n").unwrap();
        run_git(&path, ["add", "README.md"]);
        run_git(&path, ["commit", "-m", "Initial commit"]);
        run_git(&path, ["tag", "v0.1.0"]);

        fs::write(path.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").unwrap();
        run_git(&path, ["add", "Cargo.toml"]);
        run_git(&path, ["commit", "-m", "Add Cargo manifest"]);

        Self { path }
    }

    pub fn git(&self) -> Git {
        Git::new(&self.path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_git<const N: usize>(dir: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
