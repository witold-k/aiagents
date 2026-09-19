#[cfg(all(test, unix))]
mod tests {
    use aiagents::agenttools::{
        load_file::LoadFile,
        save_file::{SaveFile, SaveFileErrorType},
    };
    use fsscanner::pathfilter::Pathfilter;
    use serde_json::json;
    use std::os::unix::fs::symlink;

    #[test]
    fn load_file_rejects_symlink_escape() {
        let base = std::env::temp_dir().join(format!("aiagents-load-sandbox-{}", std::process::id()));
        let root = base.join("root");
        let outside = base.join("outside");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let secret = outside.join("secret.txt");
        std::fs::write(&secret, "secret").unwrap();
        symlink(&secret, root.join("secret.txt")).unwrap();

        let filter = Pathfilter::new(vec![root.clone()]);
        let payload = json!({"file": "secret.txt"});
        let tool = LoadFile::from_json(&root, &filter, &payload).unwrap();
        let err = tool.execute().unwrap_err();

        assert!(err.to_string().starts_with("Forbidden:"));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn save_file_rejects_existing_file_symlink_escape() {
        let base = std::env::temp_dir().join(format!("aiagents-save-sandbox-{}", std::process::id()));
        let root = base.join("root");
        let outside = base.join("outside");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let target = outside.join("target.rs");
        std::fs::write(&target, "outside").unwrap();
        symlink(&target, root.join("target.rs")).unwrap();

        let filter = Pathfilter::new(vec![root.clone()]);
        let payload = json!({"file": "target.rs", "content": "blocked", "note": "test"});
        let tool = SaveFile::from_json(&root, &filter, &payload).unwrap();
        let err = tool.execute().unwrap_err();

        assert!(matches!(err.get_err_type(), SaveFileErrorType::Forbidden));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "outside");
        std::fs::remove_dir_all(&base).unwrap();
    }
}
