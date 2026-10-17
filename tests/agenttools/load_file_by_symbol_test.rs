#[cfg(test)]
mod tests {
    use super::super::TempTree;
    use aiagents::agenttools::load_file_by_symbol::LoadFileBySymbol;
    use fsscanner::pathfilter::Pathfilter;
    use serde_json::json;

    #[test]
    fn resolves_rust_symbol_to_source() {
        let tree = TempTree::new("symbol-rust");
        tree.write(
            "Cargo.toml",
            "[package]\nname = \"resolver-test\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        tree.write("src/lib.rs", "pub struct ExampleSymbol;\n");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        let tool =
            LoadFileBySymbol::from_json(tree.root(), &filter, &json!({"symbol":"ExampleSymbol"}))
                .unwrap();
        let result = tool.execute().unwrap();
        assert!(result.data.contains("src/lib.rs"));
        assert!(result.data.contains("pub struct ExampleSymbol;"));
    }

    #[test]
    fn unreadable_candidates_are_not_disclosed() {
        let tree = TempTree::new("symbol-filter");
        tree.write(
            "Cargo.toml",
            "[package]\nname = \"resolver-test\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        tree.write("src/lib.rs", "pub struct SecretSymbol;\n");
        let allowed = tree.root().join("other");
        std::fs::create_dir_all(&allowed).unwrap();
        let filter = Pathfilter::new(vec![allowed]);
        let tool =
            LoadFileBySymbol::from_json(tree.root(), &filter, &json!({"symbol":"SecretSymbol"}))
                .unwrap();
        let result = tool.execute().unwrap();
        assert!(!result.data.contains("pub struct SecretSymbol;"));
        assert!(!result.data.contains("src/lib.rs"));
    }

    #[test]
    fn missing_symbol_is_rejected() {
        let tree = TempTree::new("symbol-decode");
        let filter = Pathfilter::new(vec![tree.root().to_path_buf()]);
        assert!(LoadFileBySymbol::from_json(tree.root(), &filter, &json!({})).is_err());
    }
}
