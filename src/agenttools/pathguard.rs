// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use fsscanner::pathfilter::Pathfilter;
use std::path::Path;

/// Validates an existing path after resolving all symbolic links.
#[inline]
pub(crate) fn can_read_resolved(filter: &Pathfilter, path: &Path) -> bool {
    filter.contains(path) && filter.contains_resolved(path)
}

/// Validates a write target without requiring the final file to exist.
///
/// The lexical check rejects `..` escapes. The nearest existing ancestor is then
/// canonicalized and checked as well, preventing writes through a directory symlink
/// that points outside the configured write root.
pub(crate) fn can_write_resolved(filter: &Pathfilter, path: &Path) -> bool {
    if !filter.can_write(path) {
        return false;
    }

    if path.exists() {
        return filter.contains_resolved(path);
    }

    let mut ancestor = path.parent();
    while let Some(candidate) = ancestor {
        if candidate.exists() {
            return filter.contains_resolved(candidate);
        }
        ancestor = candidate.parent();
    }

    false
}

