#[cfg(test)]
mod tests {
    use super::super::TempTree;
    use aiagents::agenttools::load_file_part::LoadFilePart;
    use fsscanner::pathfilter::Pathfilter;
    use serde_json::json;

    #[test]
    fn reads_requested_lines() {
        let tree = TempTree::new("load-part");
        tree.write("input.txt", "zero\none\ntwo\nthree\n");

        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"file": "input.txt", "start": 2, "count": 2});
        let tool =
            LoadFilePart::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        assert_eq!(result.data, "one\ntwo");
        assert_eq!(result.start, 1);
        assert_eq!(result.count, 2);
    }
    #[test]
    fn malformed_payload_is_rejected_before_file_access() {
        let tree = TempTree::new("load-part-decode");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);

        assert!(
            LoadFilePart::from_json(
                tree.root(),
                &filter,
                &json!({"file":"x", "start":"bad", "count":1})
            )
            .is_err()
        );
    }

}
