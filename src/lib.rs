// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

pub mod aiagentloop;
pub mod aimessage;
pub mod airequest;

pub mod generated_languages {
    include!(concat!(env!("OUT_DIR"), "/generated_languages.rs"));
}
pub mod generated_tasks {
    include!(concat!(env!("OUT_DIR"), "/generated_tasks.rs"));
}
pub mod generated_workspaces {
    include!(concat!(env!("OUT_DIR"), "/generated_workspaces.rs"));
}

pub mod agenttools;
pub mod cli;
pub mod config;
pub mod repostate;
pub mod utils;
pub mod workflows;
