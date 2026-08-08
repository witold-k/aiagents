// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::Path;

use crate::{
    workflows::buildresult::Buildresult,
    utils::doprocess::DoProcess,
};

pub fn run_cmd(path: &Path, cmd: &[String]) -> Buildresult {
    let mut lines = Vec::new();
    let proc = DoProcess::from_string_vec(cmd, path);
    let (result, _) = proc.run_to_lines_combined(&mut lines);

    Buildresult::new(result, path.to_path_buf(), path.to_path_buf(), lines)
}

