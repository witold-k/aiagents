// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Compatibility conversion from build-wrapper execution to aiagents diagnostics.

use crate::runtimetools::buildresult::Buildresult;
use build_wrapper::{CommandSpec, StageResult};
use std::path::Path;

pub fn run_cmd(path: &Path, cmd: &Option<CommandSpec>) -> Buildresult {
    let Some(cmd) = cmd else {
        return Buildresult::new(0, path.to_path_buf(), path.to_path_buf(), vec![]);
    };
    match build_wrapper::process::execute(cmd, path) {
        Ok(output) => {
            let mut lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::to_owned)
                .collect();
            lines.extend(
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .map(str::to_owned),
            );
            Buildresult::new(
                output.status.code().unwrap_or(-1),
                path.to_path_buf(),
                path.to_path_buf(),
                lines,
            )
        }
        Err(err) => Buildresult::new(
            -1,
            path.to_path_buf(),
            path.to_path_buf(),
            vec![format!("process error: {err}")],
        ),
    }
}

pub fn from_stage_result(path: &Path, stage: &StageResult) -> Buildresult {
    let mut lines: Vec<String> = stage.stdout.lines().map(str::to_owned).collect();
    lines.extend(stage.stderr.lines().map(str::to_owned));
    if let Some(error) = &stage.error {
        lines.push(format!("process error: {error}"));
    }
    Buildresult::new(
        stage.exit_code.unwrap_or(-1),
        path.to_path_buf(),
        path.to_path_buf(),
        lines,
    )
}
