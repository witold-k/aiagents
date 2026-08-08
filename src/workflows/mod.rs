// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

pub mod build;
pub mod build_lint_test;
pub mod buildresult;
pub mod buildsystem;
pub mod doc;
pub mod endless;
pub mod runbuild;
pub mod select_workflow;
pub mod setupbuild;
pub mod workspace;
pub mod generic_work_step;

pub trait Runnable {
    fn run();
}
