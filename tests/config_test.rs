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
