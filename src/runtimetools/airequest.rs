// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fmt;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use ureq::{
    Agent,
    tls::TlsConfig,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AIRequest {
    url: String,
    model: String,
    api_key: String,
    insecure: bool,
    max_tokens: u32,
    temperature: f32,
}

pub enum AIRequestResult {
    Ok(Value),
    DeserFailed(AIRequest, String),
    RequestFailed(AIRequest, String),
    DecodeFailed(AIRequest, String),
    ReportedError(AIRequest, Value, Value),
}

impl fmt::Display for AIRequestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok(val) => write!(f, "Ok: {}", val),
            Self::DeserFailed(req, err) => write!(f, "DeserFailed: {} => {}", req, err),
            Self::RequestFailed(req, err) => write!(f, "RequestFailed: {} => {}", req, err),
            Self::DecodeFailed(req, err) => write!(f, "DecodeFailed: {} => {}", req, err),
            Self::ReportedError(req, err, payload) => write!(f, "ReportedFailed: {} => {}, {}", req, err, payload),
        }
    }
}

impl AIRequest {
    pub fn new(
        model: impl Into<String>,
        url: impl Into<String>,
        api_key: impl Into<String>,
        insecure: bool,
        max_tokens: u32,
        temperature: f32,
    ) -> Self {
        AIRequest {
            url: url.into(),
            model: model.into(),
            api_key: api_key.into(),
            insecure,
            max_tokens,
            temperature,
        }
    }

    fn create_agent(&self) -> Agent {
        let tls_config = TlsConfig::builder()
            .disable_verification(self.insecure)
            .build();

        Agent::config_builder()
            .tls_config(tls_config)
            .build()
            .into()
    }

    pub fn request(&self, messages: &str) -> AIRequestResult {
        let messages: Value = match serde_json::from_str(messages) {
            Ok(msg) => msg,
            Err(err) => return AIRequestResult::DeserFailed(self.clone(), err.to_string()),
        };

        let json_payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
        });

        let agent = self.create_agent();

        let mut req = agent
            .post(&self.url)
            .header("Content-Type", "application/json");

        if !self.api_key.is_empty() {
            req = req.header(
                "Authorization",
                &format!("Bearer {}", self.api_key),
            );
        }

        let mut response = match req.send_json(json_payload.clone()) {
            Ok(resp) => resp,
            Err(err) => return AIRequestResult::RequestFailed(self.clone(), err.to_string()),
        };

        let json: Value = match response.body_mut().read_json() {
            Ok(json) => json,
            Err(err) => return AIRequestResult::DecodeFailed(self.clone(), err.to_string()),
        };

        if let Some(error) = json.get("error") {
            return AIRequestResult::ReportedError(self.clone(), error.clone(), json_payload);

        }

        AIRequestResult::Ok(json)
    }
}

impl fmt::Display for AIRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "url: {}, model: {}, max_tokens: {}, temperature {}",
            self.url, self.model, self.max_tokens, self.temperature
        )
    }
}
