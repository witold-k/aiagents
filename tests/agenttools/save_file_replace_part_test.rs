#[cfg(test)]
mod tests {
    use super::super::TempTree;
    use aiagents::agenttools::save_file_replace_part::SaveFilePart;
    use fsscanner::pathfilter::Pathfilter;
    use serde_json::json;

    #[test]
    fn replaces_selected_occurrence_only() {
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
        let tool =
            SaveFilePart::from_json(tree.root(), &filter, &payload).unwrap();

        let result = tool.execute().unwrap();
        assert_eq!(result.occurrence_index, 1);
        assert_eq!(
            std::fs::read_to_string(tree.root().join("input.txt")).unwrap(),
            "same\nseparator\nchanged\n"
        );
    }

    #[test]
    fn mismatch_leaves_file_unchanged() {
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
        let tool =
            SaveFilePart::from_json(tree.root(), &filter, &payload).unwrap();

        let err = tool.execute().unwrap_err();
        assert!(err.to_string().starts_with("OriginalMismatch:"));
        assert_eq!(
            std::fs::read_to_string(tree.root().join("input.txt")).unwrap(),
            "original data"
        );
    }
}
