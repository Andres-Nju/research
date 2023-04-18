    pub fn new_with_shell_and_path(
        arguments: ArgMatches,
        shell: Shell,
        path: PathBuf,
        logical_path: PathBuf,
    ) -> Context {
        let config = StarshipConfig::initialize();

        // Unwrap the clap arguments into a simple hashtable
        // we only care about single arguments at this point, there isn't a
        // use-case for a list of arguments yet.
        let properties: HashMap<&str, std::string::String> = arguments
            .args
            .iter()
            .filter(|(_, v)| !v.vals.is_empty())
            .map(|(a, b)| (*a, b.vals.first().cloned().unwrap().into_string().unwrap()))
            .collect();

        // Canonicalize the current path to resolve symlinks, etc.
        // NOTE: On Windows this converts the path to extended-path syntax.
        let current_dir = Context::expand_tilde(path);
        let current_dir = current_dir.canonicalize().unwrap_or(current_dir);
        let logical_dir = logical_path;

        let cmd_timeout = Duration::from_millis(config.get_root_config().command_timeout);

        Context {
            config,
            properties,
            current_dir,
            logical_dir,
            dir_contents: OnceCell::new(),
            repo: OnceCell::new(),
            shell,
            #[cfg(test)]
            env: HashMap::new(),
            #[cfg(test)]
            cmd: HashMap::new(),
            #[cfg(feature = "battery")]
            battery_info_provider: &crate::modules::BatteryInfoProviderImpl,
            cmd_timeout,
        }
    }
