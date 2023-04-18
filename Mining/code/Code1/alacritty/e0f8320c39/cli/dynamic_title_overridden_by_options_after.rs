    fn dynamic_title_overridden_by_options() {
        let mut config = UiConfig::default();

        let options = Options { title: Some("foo".to_owned()), ..Options::default() };
        options.override_config(&mut config);

        assert!(!config.window.dynamic_title);
    }
