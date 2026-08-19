// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde::{Serialize, Deserialize};
use serde_json::Value;
use ureq::Agent;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AIRequest {
    url: String,
    model: String,
    api_key: String,
    max_tokens: u32,
    temperature: f32,
}

impl AIRequest {
    pub fn new(
        model: impl Into<String>,
        url: impl Into<String>,
        api_key: impl Into<String>,
        max_tokens: u32,
        temperature: f32,
    ) -> Self {
        AIRequest {
            url: url.into(),
            model: model.into(),
            api_key: api_key.into(),
            max_tokens,
            temperature,
        }
    }

    fn create_agent(&self) -> Agent {
        let config = Agent::config_builder().build();
        config.into()
    }

    pub fn request(&self, messages: &str) -> Result<Value, String> {
        let messages: Value =
            serde_json::from_str(messages).map_err(|e| format!("Invalid messages JSON: {e}"))?;

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

        let mut response = req
            .send_json(json_payload.clone())
            .map_err(|e| format!("Request failed: {e}"))?;

        let json: Value = response
            .body_mut()
            .read_json()
            .map_err(|e| format!("Invalid JSON response: {e}"))?;

        if let Some(error) = json.get("error") {
            return Err(format!(
                "API error: {}\nPayload: {}",
                error,
                json_payload
            ));
        }

        Ok(json)
    }

}

