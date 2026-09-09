// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use std::path::{PathBuf};

#[derive(Clone)]
pub struct Buildresult {
    pub result: i32,
    pub projectpath: PathBuf,
    pub executionpath: PathBuf,
    pub output: Vec<String>,
}

impl Buildresult {
    pub fn new(
        result: i32,
        projectpath: PathBuf,
        executionpath: PathBuf,
        output: Vec<String>,
    ) -> Self {
        Self {
            result,
            projectpath,
            executionpath,
            output,
        }
    }

    pub fn new_need_build() -> Self {
        Self {
            result: 1,
            projectpath: PathBuf::new(),
            executionpath: PathBuf::new(),
            output: vec![String::new()],
        }
    }

    pub fn new_no_build() -> Self {
        Self {
            result: 0,
            projectpath: PathBuf::new(),
            executionpath: PathBuf::new(),
            output: vec![String::new()],
        }
    }

    pub fn has_error(&self) -> bool {
        self.result != 0
    }

    pub fn is_dummy(&self) -> bool {
        self.projectpath.as_os_str().is_empty()
    }

    pub fn limit_lines(&self, line_count: usize) -> Self {
        let mut clone = self.clone();
        clone.output = self.output.iter().take(line_count).cloned().collect();
        clone
    }
}

impl fmt::Display for Buildresult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "result: {} {}", self.result, if self.has_error() { " => has_error" } else { " => valid " })?;
        writeln!(f, "in:  {}", self.projectpath.display())?;
        writeln!(f, "out: {}", self.executionpath.display())?;
        for line in &self.output {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Buildresult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

