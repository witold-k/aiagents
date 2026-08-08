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
        // Build JSON payload manually, same as reqwest version
        let mut payload = String::with_capacity(4096);

        payload.push_str("{\"model\":\"");
        payload.push_str(&self.model);
        payload.push_str("\",\"messages\":");
        payload.push_str(messages);
        payload.push_str(",\"max_tokens\":");
        payload.push_str(&self.max_tokens.to_string());
        payload.push_str(",\"temperature\":");
        payload.push_str(&self.temperature.to_string());
        payload.push('}');

        // Parse into serde_json::Value for send_json()
        let json_payload: Value =
            serde_json::from_str(&payload).map_err(|e| e.to_string())?;

        let agent = self.create_agent();

        let mut req = agent
            .post(&self.url)
            .header("Content-Type", "application/json");

        if !self.api_key.is_empty() {
            req = req.header("Authorization", &format!("Bearer {}", self.api_key));
        }

        // send_json() returns Result<Response<Body>, Error> in ureq 3.x
        let mut response = req
            .send_json(json_payload)
            .map_err(|e| e.to_string())?;

        // Extract JSON body via body_mut().read_json() in ureq 3.x
        let json: Value = response
            .body_mut()
            .read_json()
            .map_err(|e| e.to_string())?;

        if json.get("error").is_some() {
            return Err(format!("Error: {}\nPayload: {}", json, payload));
        }

        Ok(json)
    }
}

