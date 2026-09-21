use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct TempTree {
    base: PathBuf,
    root: PathBuf,
}

impl TempTree {
    fn new(name: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "aiagents-file-access-{}-{}-{}",
            name,
            std::process::id(),
            id
        ));
        let root = base.join("root");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&root).unwrap();
        Self { base, root }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn write(&self, rel: &str, content: &str) -> PathBuf {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        path
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

pub mod list_dir_test;
pub mod load_file_part_test;
pub mod load_file_test;
pub mod pathguard_test;
pub mod save_file_replace_part_test;
pub mod save_file_test;
pub mod scan_dir_test;
