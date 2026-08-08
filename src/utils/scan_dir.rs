// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use globset::{Glob, GlobSet, GlobSetBuilder};
use std::fs;
use std::path::{Path, PathBuf};
use fsscanner::pathfilter::{Pathfilter};

pub fn scan_with_globset_and_filter(
    root: impl AsRef<Path>,
    pattern: &[&str],
    filter: &Pathfilter,
) -> Vec<PathBuf> {
    // Build GlobSet (supports brace expansion)
    let mut builder = GlobSetBuilder::new();
    for pat in pattern {
        let pat = pat.trim();
        if !pat.is_empty() {
            builder.add(Glob::new(pat).unwrap());
        }
    }
    let set: GlobSet = builder.build().unwrap();

    let mut results = Vec::new();
    walk_recursive(root.as_ref(), &set, filter, &mut results);
    results
}

fn walk_recursive(
    dir: &Path,
    set: &GlobSet,
    filter: &Pathfilter,
    results: &mut Vec<PathBuf>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() && filter.contains(&path) {
                walk_recursive(&path, set, filter, results);
                continue;
            }

            // Match using globset
            if set.is_match(&path)
                && let Ok(canon) = path.canonicalize()
                    && filter.contains(&canon) {
                        results.push(canon);
                    }
        }
    }
}

