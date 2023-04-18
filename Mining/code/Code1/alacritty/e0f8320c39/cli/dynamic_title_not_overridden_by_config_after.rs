    fn dynamic_title_not_overridden_by_config() {
        let mut config = UiConfig::default();

        config.window.title = "foo".to_owned();
        Options::default().override_config(&mut config);

        assert!(config.window.dynamic_title);
    }
