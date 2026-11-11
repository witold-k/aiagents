use aiagents::config::{AIProvider, Config};


#[test]
fn duplicate_provider_names_are_rejected() {
    let mut config = Config::default();
    config.providerlist.push(AIProvider {
        name: config.providerlist[0].name.clone(),
        ..AIProvider::default()
    });

    let dir = unique_config_dir("duplicate");
    let path = dir.join("config.json");
    config.save(&path).unwrap();

    let error = match Config::load(&path) {
        Ok(_) => panic!("duplicate provider name was accepted"),
        Err(error) => error,
    };
    assert!(format!("{error:?}").contains("duplicate provider name"));

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn default_provider_names_are_unique() {
    let config = Config::default();
    let mut names = std::collections::HashSet::new();

    assert!(
        config
            .providerlist
            .iter()
            .all(|provider| names.insert(provider.name.as_str()))
    );
}

#[test]
fn config_filters_expand_home_and_project_fragments() {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home).canonicalize().unwrap();
    let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
    let mut config = aiagents::config::Config::default();
    config.writefilter = vec!["{{home}}/.project".into()];
    config.readfilter = vec!["{{projectdir}}/..".into()];
    let expanded = config.with_expanded_filters().unwrap();
    assert_eq!(
        expanded.writefilter[0],
        format!("{}/.project", home.display())
    );
    assert_eq!(expanded.readfilter[0], format!("{}/..", cwd.display()));
}

#[test]
fn config_filters_reject_unknown_placeholders() {
    let mut config = aiagents::config::Config::default();
    config.readfilter = vec!["{{unknown}}".into()];
    assert!(config.with_expanded_filters().is_err());
}

#[test]
fn default_http_retry_policy_and_json_override() {
    use aiagents::config::Config;
    let default = Config::default();
    assert_eq!(default.http_retry.count, 3);
    assert_eq!(default.http_retry.delay_seconds, 30);
    let mut json = serde_json::to_value(&default).unwrap();
    json.as_object_mut().unwrap().remove("http_retry");
    let legacy: Config = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(legacy.http_retry.count, 3);
    assert_eq!(legacy.http_retry.delay_seconds, 30);
    json["http_retry"] = serde_json::json!({"count": 2, "delay_seconds": 5});
    let custom: Config = serde_json::from_value(json).unwrap();
    assert_eq!(custom.http_retry.count, 2);
    assert_eq!(custom.http_retry.delay_seconds, 5);
}

fn unique_config_dir(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "aiagents-config-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}

fn write_overlay(dir: &std::path::Path, name: &str, value: serde_json::Value) {
    std::fs::write(dir.join(name), serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

#[test]
fn merges_partial_overlays_in_sorted_filename_order() {
    use aiagents::config::Config;
    let dir = unique_config_dir("overlays");
    let main = dir.join("config.json");
    let mut initial = Config::default();
    initial.provider = "external".to_owned();
    // The main config is intentionally invalid alone: the provider comes
    // from a sidecar, so validation must happen after merging.
    initial.save(&main).unwrap();
    write_overlay(&dir, "10-models.json", serde_json::json!({
        "providerlist": [{
            "name": "external", "active": true, "endpoint": "http://first",
            "model": "first", "comment": "", "source": "", "api_key": "",
            "insecure": false, "llmbin": "", "llmmodeldir": "",
            "llmparam": []
        }],
        "taskproviderlist": [{"task": "extract", "provider": "external", "temperature": 0.3}],
        "max_try_count": {"max_workflow_fail": 3}
    }));
    write_overlay(&dir, "20-overrides.json", serde_json::json!({
        "providerlist": [{"name": "external", "endpoint": "http://second"}],
        "taskproviderlist": [{"task": "extract", "temperature": 0.8}],
        "max_try_count": {"max_tool_call_fail": 4},
        "queue_length_max": 30,
        "readfilter": ["{{projectdir}}"]
    }));
    std::fs::write(dir.join("ignore.txt"), "{ not JSON }").unwrap();
    let loaded = Config::load(&main).unwrap();
    assert_eq!(loaded.provider, "external");
    assert_eq!(loaded.get_provider("external").unwrap().endpoint, "http://second");
    assert_eq!(loaded.get_provider("external").unwrap().model, "first");
    assert_eq!(loaded.taskproviderlist.len(), 1);
    assert_eq!(loaded.taskproviderlist[0].temperature, 0.8);
    assert_eq!(loaded.max_try_count.max_workflow_fail, 3);
    assert_eq!(loaded.max_try_count.max_tool_call_fail, 4);
    assert_eq!(loaded.queue_length_max, 30);
    assert_eq!(loaded.readfilter.len(), 1);
    assert!(!loaded.readfilter[0].contains("{{projectdir}}"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn loads_default_main_config_without_overlays() {
    use aiagents::config::Config;
    let dir = unique_config_dir("legacy");
    let main = dir.join("config.json");
    Config::default().save(&main).unwrap();
    let config = Config::load(&main).unwrap();
    assert_eq!(config.provider, Config::default().provider);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn creates_main_config_and_merges_existing_sidecar() {
    use aiagents::config::Config;
    let dir = unique_config_dir("first-run");
    let main = dir.join("config.json");
    write_overlay(&dir, "extra.json", serde_json::json!({"queue_length_max": 99}));
    let loaded = Config::load_or_create(Some(main.to_string_lossy().into_owned())).unwrap();
    assert!(main.is_file());
    assert_eq!(loaded.queue_length_max, 99);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_malformed_overlay_with_its_path() {
    use aiagents::config::Config;
    let dir = unique_config_dir("malformed");
    let main = dir.join("config.json");
    Config::default().save(&main).unwrap();
    std::fs::write(dir.join("bad.json"), "{ nope").unwrap();
    let error = Config::load(&main).err().expect("must reject invalid JSON");
    assert!(format!("{error:?}").contains("bad.json"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_duplicate_provider_in_single_overlay() {
    use aiagents::config::Config;
    let dir = unique_config_dir("duplicate-overlay");
    let main = dir.join("config.json");
    Config::default().save(&main).unwrap();
    write_overlay(&dir, "duplicate.json", serde_json::json!({
        "providerlist": [{"name":"duplicate"}, {"name":"duplicate"}]
    }));
    let error = Config::load(&main).err().expect("must reject duplicates");
    assert!(format!("{error:?}").contains("duplicate provider name"));
    std::fs::remove_dir_all(dir).unwrap();
}
