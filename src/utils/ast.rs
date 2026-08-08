// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

pub fn get_ast_string(path: &str) -> String {
     match std::process::Command::new("ast-outline")
        .arg(path)
        .output()
    {
        Ok(out) => {
            String::from_utf8_lossy(&out.stdout).to_string()
        }
        Err(_) => {
            "".into()
        }
    }
}
