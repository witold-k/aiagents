#[cfg(test)]
mod tests {
    use super::super::TempTree;
    use aiagents::agenttools::save_file::{SaveFile, SaveFileErrorType};
    use fsscanner::pathfilter::Pathfilter;
    use serde_json::json;

    #[test]
    fn creates_and_truncates_file() {
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
    fn creates_new_file_in_existing_directory() {
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
            std::fs::read_to_string(
                tree.root().join("nested/new.txt")
            )
            .unwrap(),
            "created"
        );
    }

    #[test]
    fn rejects_empty_content_without_modifying_existing_file() {
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
        assert!(matches!(
            err.get_err_type(),
            SaveFileErrorType::EmptyContent
        ));
        assert_eq!(
            std::fs::read_to_string(tree.root().join("output.txt")).unwrap(),
            "keep me"
        );
    }

    #[test]
    fn malformed_payload_is_rejected_before_file_access() {
        let tree = TempTree::new("save-decode");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);

        assert!(
            SaveFile::from_json(
                tree.root(),
                &filter,
                &json!({"file":"x", "content":"data"})
            )
            .is_err()
        );
    }
}
