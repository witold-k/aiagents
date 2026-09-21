#[cfg(test)]
mod tests {
    use super::super::TempTree;
    use aiagents::agenttools::load_file::LoadFile;
    use fsscanner::pathfilter::Pathfilter;
    use serde_json::json;

    #[test]
    fn reads_allowed_file() {
        let tree = TempTree::new("load");
        tree.write("nested/input.txt", "alpha\nbeta\n");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"file": "nested/input.txt"});
        let tool = LoadFile::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        assert_eq!(result.data, "alpha\nbeta\n");
    }

    #[test]
    fn reports_missing_file() {
        let tree = TempTree::new("load-missing");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"file": "missing.txt"});

        let err =
            LoadFile::from_json(tree.root(), &filter, &payload).unwrap_err();
        assert!(err.to_string().starts_with("NotFound:"));
    }
    #[test]
    fn malformed_payload_is_rejected_before_file_access() {
        let tree = TempTree::new("load-decode");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);

        assert!(
            LoadFile::from_json(tree.root(), &filter, &json!({})).is_err()
        );
    }

}
