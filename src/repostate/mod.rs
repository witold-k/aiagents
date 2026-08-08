// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! this module contain methods to save the current
//! state of files (do a backup) and restore it
//! for such operation e.g. git could be used => see `GitState`:
//! - for save state: `git add -A`
//! - for restore state: `git checkout .`
//!
//! with this functionality a fault state that was caused by
//! llm/ai action => see `agenttools`/`workflows` could be restored
//! to an old working or less damanged one

pub mod gitstate;

pub trait RepoState {
    fn commit(&self) -> bool;

    fn restore(&self) -> bool;
}
