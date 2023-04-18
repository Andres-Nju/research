    fn default() -> Self {
        StyleSystemOptions {
            disable_style_sharing_cache: get_env("DISABLE_STYLE_SHARING_CACHE"),
            dump_style_statistics: get_env("DUMP_STYLE_STATISTICS"),
        }
    }
