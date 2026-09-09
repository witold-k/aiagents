// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::Path;
use crate::agenttools::all_tools::ToolOutput;
use crate::runtimetools::{
    buildresult::Buildresult,
};

pub trait RunBuild {
    fn execute(
        &self,
        cb: &mut dyn FnMut(&str, &Path, &Path, &Buildresult) -> ToolOutput,
    ) -> Buildresult;
}

