// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Workflow orchestration for coding tasks.
//!
//! A workflow defines the sequence of steps required to complete a task.
//! It may coordinate one or more agents, verification steps such as
//! build/lint/test, repository recovery, and other task-specific actions.
//!
//! Workflows are responsible for deciding what should happen next.
//! [`AIAgentLoop`](crate::aiagentloop::AIAgentLoop) provides the execution
//! mechanism for an individual LLM-driven agent, while workflows provide
//! the higher-level orchestration around agents and other operations.
//!
//! A workflow should remain focused on task orchestration and should not
//! duplicate the implementation details of the components it coordinates.
//!
//! For example, a coding workflow might conceptually perform:
//!
//! ```text
//! codify → build/lint/test → review → build/lint/test → document → done
//! ```
//!
//! The individual steps may be simple or may involve another agent.
//! The workflow decides how these steps are connected and what happens
//! when a step succeeds or fails.

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
