#[cfg(test)]
mod tests {
    use aiagents::config::{AIProvider, Config};

    #[test]
    fn duplicate_provider_names_are_rejected() {
        let mut config = Config::default();
        config.providerlist.push(AIProvider {
            name: config.providerlist[0].name.clone(),
            ..AIProvider::default()
        });

        let path = std::env::temp_dir().join(format!(
            "aiagents-config-duplicate-provider-{}.json",
            std::process::id()
        ));
        config.save(&path).unwrap();

        let error = match Config::load(&path) {
            Ok(_) => panic!("duplicate provider name was accepted"),
            Err(error) => error,
        };
        assert!(format!("{error:?}").contains("duplicate provider name"));

        std::fs::remove_file(path).unwrap();
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
