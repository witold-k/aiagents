// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use super::WorkflowSelector;
    use crate::generated_tasks::Tasks;

    #[test]
    fn chunked_text_transformation_does_not_require_language() {
        assert!(!WorkflowSelector::requires_language(
            Tasks::ExtractMessageTables
        ));
    }

    #[test]
    fn code_workflows_require_language() {
        assert!(WorkflowSelector::requires_language(Tasks::FixCode));
        assert!(WorkflowSelector::requires_language(Tasks::GenCode));
    }
