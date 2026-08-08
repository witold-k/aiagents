// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::Path;
use crate:: {
    agenttools::all_tools::ToolOutput,
    workflows::buildresult::Buildresult,
};

pub trait RunBuild {
    fn execute(
        &self,
        cb: &mut dyn FnMut(&str, &Path, &Path, &Buildresult) -> ToolOutput,
    ) -> Buildresult;
}


