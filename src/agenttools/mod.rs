// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! This module contains a selection of tools that the agent (LLM/AI)
//! may use. This is the only possibility for the AI to interact
//! with the outer world/modify code.
//! In general: IA/LLM *MUST NEVER* have access to
//! - version control systems, e.g. git, pijul, svn, ...
//! - never modify files/dirs outside current project => see `PathFilter`
//! - never read files/dirs outside allowed filter => see `PathFiler`
//! - never execute direct or indirect other tools that are listed here
//! - never has sudo or higher priviledged access then the current user

pub mod aitooltype;
pub mod all_tools;
pub mod ast;
pub mod add_note;
pub mod done;
pub mod failed;
pub mod list_dir;
pub mod load_file;
pub mod load_file_part;
pub mod save_file;
pub mod save_file_replace_part;
pub mod scan_dir;
pub mod set_focus;
pub mod valid;
