// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde_json::Value;

pub fn get_json_field(payload: &Value, name: &str) -> Result<String, String> {
    match payload.get(name) {
        Some(content) => {
            match content.as_str() {
                Some(text) => Ok(text.to_string()),
                None       => {
                    let e = format!("got field {}, but it is empty:\n{}", name, payload);
                    Err(e)
                }
            }
        }
        None => {
            let e = format!("field {} not found:\n{}", name, payload);
            Err(e)
        }
    }
}
