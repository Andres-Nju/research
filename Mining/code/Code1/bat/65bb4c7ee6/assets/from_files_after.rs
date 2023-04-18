    pub fn from_files(dir: Option<&Path>) -> Result<Self> {
        let source_dir = dir.unwrap_or_else(|| PROJECT_DIRS.config_dir());

        let theme_dir = source_dir.join("themes");
        let theme_set = ThemeSet::load_from_folder(&theme_dir).chain_err(|| {
            format!(
                "Could not load themes from '{}'",
                theme_dir.to_string_lossy()
            )
        })?;
        let mut syntax_set = SyntaxSet::new();
        let syntax_dir = source_dir.join("syntaxes");
        if !syntax_dir.exists() {
            return Err(format!(
                "Could not load syntaxes from '{}'",
                syntax_dir.to_string_lossy()
            ).into());
        }
        let _ = syntax_set.load_syntaxes(syntax_dir, true);
        syntax_set.load_plain_text_syntax();

        Ok(HighlightingAssets {
            syntax_set,
            theme_set,
        })
    }
