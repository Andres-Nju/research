    fn default() -> Self {
        StyleSystemOptions {
            disable_style_sharing_cache:
                // Disable the style sharing cache on opt builds until
                // bug 1358693 is fixed, but keep it on debug builds to make
                // sure we don't introduce correctness bugs.
                if cfg!(debug_assertions) { get_env("DISABLE_STYLE_SHARING_CACHE") } else { true },
            dump_style_statistics: get_env("DUMP_STYLE_STATISTICS"),
        }
    }
