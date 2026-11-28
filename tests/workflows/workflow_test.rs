// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::workflows::workflow::WorkflowResult;

#[test]
fn explicit_workflow_errors_are_terminal() {
    let result = WorkflowResult::Error("checkpoint conflict".into());
    assert!(!result.success());
    assert!(result.is_terminal_failure());
    assert!(!WorkflowResult::Ok.is_terminal_failure());
}
