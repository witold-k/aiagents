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


#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    fn sandbox() -> (PathBuf, PathBuf, Pathfilter) {
        let base = std::env::temp_dir().join(format!(
            "aiagents-pathguard-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let root = base.join("root");
        let outside = base.join("outside");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let filter = Pathfilter::new(vec![root.clone()]);
        (root, outside, filter)
    }

    #[test]
    fn read_rejects_file_symlink_escape() {
        let (root, outside, filter) = sandbox();
        let secret = outside.join("secret.txt");
        std::fs::write(&secret, "secret").unwrap();
        let link = root.join("secret.txt");
        symlink(&secret, &link).unwrap();

        assert!(!can_read_resolved(&filter, &link));

        std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn write_rejects_existing_file_symlink_escape() {
        let (root, outside, filter) = sandbox();
        let target = outside.join("target.txt");
        std::fs::write(&target, "outside").unwrap();
        let link = root.join("target.txt");
        symlink(&target, &link).unwrap();

        assert!(!can_write_resolved(&filter, &link));

        std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn write_rejects_new_file_below_directory_symlink_escape() {
        let (root, outside, filter) = sandbox();
        let link = root.join("out");
        symlink(&outside, &link).unwrap();

        assert!(!can_write_resolved(&filter, &link.join("new.rs")));

        std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn write_allows_new_file_below_real_sandbox_directory() {
        let (root, _outside, filter) = sandbox();
        let dir = root.join("src");
        std::fs::create_dir_all(&dir).unwrap();

        assert!(can_write_resolved(&filter, &dir.join("new.rs")));

        std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }
}
