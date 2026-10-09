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
        assert!(
            format!("{error:?}").contains("duplicate provider name")
        );

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
    let Some(home) = std::env::var_os("HOME") else { return };
    let home = std::path::PathBuf::from(home).canonicalize().unwrap();
    let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
    let mut config = aiagents::config::Config::default();
    config.writefilter = vec!["{{home}}/.project".into()];
    config.readfilter = vec!["{{projectdir}}/..".into()];
    let expanded = config.with_expanded_filters().unwrap();
    assert_eq!(expanded.writefilter[0], format!("{}/.project", home.display()));
    assert_eq!(expanded.readfilter[0], format!("{}/..", cwd.display()));
}

#[test]
fn config_filters_reject_unknown_placeholders() {
    let mut config = aiagents::config::Config::default();
    config.readfilter = vec!["{{unknown}}".into()];
    assert!(config.with_expanded_filters().is_err());
}
