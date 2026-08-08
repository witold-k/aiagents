// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::{fs, path::Path, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let visible_root = PathBuf::from("generated_src");

    fs::create_dir_all(&visible_root).unwrap();

    let root = PathBuf::from("task_descriptions");

    // Loop over all entries inside task_descriptions
    for entry in fs::read_dir(&root).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            let folder_name = path.file_name().unwrap().to_str().unwrap();
            generate_dimension(folder_name, &path, &out_dir, &visible_root);
        }
    }
}

fn generate_dimension(
    folder_name: &str,
    folder_path: &PathBuf,
    out_dir: &Path,
    visible_root: &Path,
) {
    // Enum name = PascalCase of folder name
    let enum_name = folder_name
        .split('_')
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<String>();

    let filepart_name = enum_name.to_lowercase();
    let big_name = enum_name.to_uppercase();

    let mut enum_variants = String::new();
    let mut match_as_str = String::new();
    let mut match_from_str = String::new();
    let mut match_content = String::new();
    let mut all_list = String::new();

    if let Ok(entries) = fs::read_dir(folder_path) {
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_file() {
                let stem = path.file_stem().unwrap().to_str().unwrap();

                let variant = stem
                    .split('_')
                    .map(|s| {
                        let mut c = s.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    })
                    .collect::<String>();

                let content = fs::read_to_string(&path).unwrap();

                enum_variants.push_str(&format!("    {},\n", variant));
                match_as_str.push_str(&format!("            {enum_name}::{variant} => \"{stem}\",\n"));
                match_from_str.push_str(&format!("            \"{stem}\" => Some({enum_name}::{variant}),\n"));
                match_content.push_str(&format!("            {enum_name}::{variant} => r#\"{content}\"#,\n"));
                all_list.push_str(&format!("    {enum_name}::{variant},\n"));
            }
        }
    }

    let out = format!(
        r#"use serde::{{Serialize, Deserialize}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum {enum_name} {{
{enum_variants}
}}

pub const ALL_{big_name}: &[{enum_name}] = &[
{all_list}
];

impl {enum_name} {{
    pub fn as_str(&self) -> &'static str {{
        match self {{
{match_as_str}        }}
    }}

    pub fn iter() -> impl Iterator<Item = {enum_name}> {{
        ALL_{big_name}.iter().copied()
    }}

    pub fn len() -> usize {{
        ALL_{big_name}.len()
    }}

    pub fn to_vec_str() -> Vec<&'static str> {{
        ALL_{big_name}.iter().map(|t| t.as_str()).collect()
    }}

    pub fn parse(name: &str) -> Option<{enum_name}> {{
        match name {{
{match_from_str}        _ => None,
        }}
    }}

    pub fn get_prompt(&self) -> &'static str {{
        match self {{
    {match_content}    }}
}}
}}"#,
    );

    let dest_out = out_dir.join(format!("generated_{filepart_name}.rs"));
    fs::write(&dest_out, &out).unwrap();

    let dest_visible = visible_root.join(format!("generated_{filepart_name}.rs"));
    let _ = fs::remove_file(&dest_visible);

    create_link_or_copy(&dest_out, &dest_visible);
}

#[cfg(unix)]
fn create_link_or_copy(src: &std::path::Path, dst: &std::path::Path) {
    use std::os::unix::fs::symlink;
    symlink(src, dst).unwrap();
}

#[cfg(windows)]
fn create_link_or_copy(src: &std::path::Path, dst: &std::path::Path) {
    fs::copy(src, dst).unwrap();
}

