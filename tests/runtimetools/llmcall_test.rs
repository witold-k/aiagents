// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::runtimetools::llmcall::LlmCall;
use llmhttpclient::{AIRequest, AIRequestResult};

#[test]
fn only_gateway_timeout_is_retryable() {
    let request = AIRequest::new("test", "http://localhost", "", false, 100, 0.0);
    for message in ["http status: 504", "HTTP status 504", "504 Gateway Timeout"] {
        assert!(LlmCall::is_gateway_timeout(
            &AIRequestResult::RequestFailed(request.clone(), message.into())
        ));
    }
    for message in [
        "http status: 500",
        "http status: 429",
        "connection refused",
        "5040",
    ] {
        assert!(!LlmCall::is_gateway_timeout(
            &AIRequestResult::RequestFailed(request.clone(), message.into())
        ));
    }
    assert!(!LlmCall::is_gateway_timeout(
        &AIRequestResult::DecodeFailed(request, "http status: 504".into())
    ));
}
