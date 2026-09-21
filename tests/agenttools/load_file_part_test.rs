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

    #[test]
    fn rejects_zero_start() {
        let tree = TempTree::new("load-part-zero-start");
        tree.write("input.txt", "one\ntwo\n");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"file": "input.txt", "start": 0, "count": 1});

        assert!(
            LoadFilePart::from_json(tree.root(), &filter, &payload).is_err()
        );
    }

    #[test]
    fn rejects_negative_start_and_count() {
        let tree = TempTree::new("load-part-negative");
        tree.write("input.txt", "one\ntwo\n");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);

        let start = json!({"file": "input.txt", "start": -1, "count": 1});
        assert!(
            LoadFilePart::from_json(tree.root(), &filter, &start).is_err()
        );

        let count = json!({"file": "input.txt", "start": 1, "count": -1});
        assert!(
            LoadFilePart::from_json(tree.root(), &filter, &count).is_err()
        );
    }

    #[test]
    fn zero_count_returns_empty_data() {
        let tree = TempTree::new("load-part-zero-count");
        tree.write("input.txt", "one\ntwo\n");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"file": "input.txt", "start": 1, "count": 0});
        let tool =
            LoadFilePart::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        assert_eq!(result.data, "");
        assert_eq!(result.count, 0);
    }

    #[test]
    fn start_beyond_eof_returns_padding() {
        let tree = TempTree::new("load-part-eof");
        tree.write("input.txt", "one\ntwo\n");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let payload = json!({"file": "input.txt", "start": 4, "count": 2});
        let tool =
            LoadFilePart::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        assert_eq!(result.data, "\n\n\n");
    }

}
