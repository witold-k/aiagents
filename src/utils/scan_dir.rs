// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::fs;
use std::path::{Path, PathBuf};
use fsscanner::pathfilter::{Pathfilter};

pub fn scan_with_suffix_and_filter(
    root: impl AsRef<Path>,
    suffix_array: &[&str],
    filter: &Pathfilter,
) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_recursive(root.as_ref(), suffix_array, filter, &mut results);
    results
}

fn walk_recursive(
    dir: &Path,
    suffix_array: &[&str],
    filter: &Pathfilter,
    results: &mut Vec<PathBuf>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let canon = match entry.path().canonicalize() {
                Ok(canon) => canon,
                Err(_)    => continue,
            };

            if canon.is_dir() && filter.contains(&canon) {
                walk_recursive(&canon, suffix_array, filter, results);
                continue;
            }

            let Some(suffix) = canon.extension().and_then(|s| s.to_str()) else {
                continue;
            };

            if suffix_array.contains(&suffix) && filter.contains(&canon) {
                results.push(canon);
            }
        }
    }
}

