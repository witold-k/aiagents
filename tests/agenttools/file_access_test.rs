#[cfg(test)]
mod tests {
    use aiagents::agenttools::{
        list_dir::ListDir,
        load_file::LoadFile,
        load_file_part::LoadFilePart,
        save_file::{SaveFile, SaveFileErrorType},
        save_file_replace_part::SaveFilePart,
        scan_dir::ScanDir,
    };
    use fsscanner::pathfilter::Pathfilter;
    use serde_json::json;
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

    #[test]
    fn load_file_reads_allowed_file() {
        let tree = TempTree::new("load");
        tree.write("nested/input.txt", "alpha\nbeta\n");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"file": "nested/input.txt"});
        let tool = LoadFile::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        assert_eq!(result.data, "alpha\nbeta\n");
    }

    #[test]
    fn load_file_reports_missing_file() {
        let tree = TempTree::new("load-missing");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"file": "missing.txt"});
        let err = LoadFile::from_json(tree.root(), &filter, &payload).unwrap_err();
        assert!(err.to_string().starts_with("NotFound:"));
    }

    #[test]
    fn load_file_part_reads_requested_lines() {
        let tree = TempTree::new("load-part");
        tree.write("input.txt", "zero\none\ntwo\nthree\n");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"file": "input.txt", "start": 2, "count": 2});
        let tool = LoadFilePart::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        assert_eq!(result.data, "one\ntwo");
        assert_eq!(result.start, 1);
        assert_eq!(result.count, 2);
    }

    #[test]
    fn save_file_creates_and_truncates_file() {
        let tree = TempTree::new("save");
        tree.write("output.txt", "old content that is longer");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({
            "file": "output.txt",
            "content": "new",
            "note": "replace test content"
        });
        let tool = SaveFile::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        assert_eq!(result.path, tree.root().join("output.txt"));
        assert_eq!(
            std::fs::read_to_string(tree.root().join("output.txt")).unwrap(),
            "new"
        );
    }

    #[test]
    fn save_file_creates_new_file_in_existing_directory() {
        let tree = TempTree::new("save-new");
        std::fs::create_dir_all(tree.root().join("nested")).unwrap();

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({
            "file": "nested/new.txt",
            "content": "created",
            "note": "create test file"
        });
        let tool = SaveFile::from_json(tree.root(), &filter, &payload).unwrap();

        tool.execute().unwrap();
        assert_eq!(
            std::fs::read_to_string(tree.root().join("nested/new.txt")).unwrap(),
            "created"
        );
    }

    #[test]
    fn save_file_rejects_empty_content_without_modifying_existing_file() {
        let tree = TempTree::new("save-empty");
        tree.write("output.txt", "keep me");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({
            "file": "output.txt",
            "content": "",
            "note": "must fail"
        });
        let tool = SaveFile::from_json(tree.root(), &filter, &payload).unwrap();

        let err = tool.execute().unwrap_err();
        assert!(matches!(err.get_err_type(), SaveFileErrorType::EmptyContent));
        assert_eq!(
            std::fs::read_to_string(tree.root().join("output.txt")).unwrap(),
            "keep me"
        );
    }

    #[test]
    fn save_file_part_replaces_selected_occurrence_only() {
        let tree = TempTree::new("replace-part");
        tree.write("input.txt", "same\nseparator\nsame\n");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({
            "file": "input.txt",
            "index": 1,
            "original": "same",
            "content": "changed",
            "note": "replace second occurrence"
        });
        let tool = SaveFilePart::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        assert_eq!(result.occurrence_index, 1);
        assert_eq!(
            std::fs::read_to_string(tree.root().join("input.txt")).unwrap(),
            "same\nseparator\nchanged\n"
        );
    }

    #[test]
    fn save_file_part_mismatch_leaves_file_unchanged() {
        let tree = TempTree::new("replace-mismatch");
        tree.write("input.txt", "original data");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({
            "file": "input.txt",
            "index": 0,
            "original": "not present",
            "content": "changed",
            "note": "must fail"
        });
        let tool = SaveFilePart::from_json(tree.root(), &filter, &payload).unwrap();

        let err = tool.execute().unwrap_err();
        assert!(err.to_string().starts_with("OriginalMismatch:"));
        assert_eq!(
            std::fs::read_to_string(tree.root().join("input.txt")).unwrap(),
            "original data"
        );
    }

    #[test]
    fn list_dir_lists_real_directory_entries() {
        let tree = TempTree::new("list-dir");
        tree.write("a.txt", "a");
        tree.write("nested/b.txt", "b");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"path": "."});
        let tool = ListDir::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        let entries: Vec<_> = result.data.lines().collect();
        assert!(entries.iter().any(|entry| entry.ends_with("a.txt")));
        assert!(entries.iter().any(|entry| entry.ends_with("nested")));
    }

    #[test]
    fn list_dir_on_file_reports_read_failed() {
        let tree = TempTree::new("list-file");
        tree.write("plain.txt", "data");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"path": "plain.txt"});
        let tool = ListDir::from_json(tree.root(), &filter, &payload).unwrap();

        let err = tool.execute().unwrap_err();
        assert!(err.to_string().starts_with("ReadFailed:"));
    }

    #[test]
    fn scan_dir_lists_real_directory_entries() {
        let tree = TempTree::new("scan-dir");
        tree.write("first.txt", "1");
        tree.write("sub/second.txt", "2");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"path": "."});
        let tool = ScanDir::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        let entries: Vec<_> = result.data.lines().collect();
        assert!(entries.iter().any(|entry| entry.ends_with("first.txt")));
        assert!(entries.iter().any(|entry| entry.ends_with("sub")));
    }

    #[test]
    fn malformed_payloads_are_rejected_before_file_access() {
        let tree = TempTree::new("decode");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);

        assert!(LoadFile::from_json(tree.root(), &filter, &json!({})).is_err());
        assert!(LoadFilePart::from_json(
            tree.root(),
            &filter,
            &json!({"file":"x", "start":"bad", "count":1})
        )
        .is_err());
        assert!(SaveFile::from_json(
            tree.root(),
            &filter,
            &json!({"file":"x", "content":"data"})
        )
        .is_err());
        assert!(ListDir::from_json(tree.root(), &filter, &json!({})).is_err());
        assert!(ScanDir::from_json(tree.root(), &filter, &json!({})).is_err());
    }
}
