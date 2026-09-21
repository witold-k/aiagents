#[cfg(test)]
mod tests {
    use super::super::TempTree;
    use aiagents::agenttools::list_dir::ListDir;
    use fsscanner::pathfilter::Pathfilter;
    use serde_json::json;

    #[test]
    fn lists_real_directory_entries() {
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
    fn file_path_reports_read_failed() {
        let tree = TempTree::new("list-file");
        tree.write("plain.txt", "data");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"path": "plain.txt"});
        let tool = ListDir::from_json(tree.root(), &filter, &payload).unwrap();

        let err = tool.execute().unwrap_err();
        assert!(err.to_string().starts_with("ReadFailed:"));
    }
    #[test]
    fn malformed_payload_is_rejected_before_file_access() {
        let tree = TempTree::new("list-decode");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);

        assert!(
            ListDir::from_json(tree.root(), &filter, &json!({})).is_err()
        );
    }

}
