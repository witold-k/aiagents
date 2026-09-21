#[cfg(test)]
mod tests {
    use super::super::TempTree;
    use aiagents::agenttools::scan_dir::ScanDir;
    use fsscanner::pathfilter::Pathfilter;
    use serde_json::json;

    #[test]
    fn lists_real_directory_entries() {
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
    fn malformed_payload_is_rejected_before_file_access() {
        let tree = TempTree::new("scan-decode");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);

        assert!(
            ScanDir::from_json(tree.root(), &filter, &json!({})).is_err()
        );
    }

}
