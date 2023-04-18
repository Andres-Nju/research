    pub async fn reload_config(&self, cfg_path: &ConfigPath) -> Result<(), ShellError> {
        trace!("Reloading cfg {:?}", cfg_path);

        let mut configs = self.configs.lock();
        let cfg = match cfg_path {
            ConfigPath::Global(path) => {
                configs.global_config.iter_mut().find(|cfg| &cfg.file_path == path).ok_or_else(||
                        ShellError::labeled_error(
                            &format!("Error reloading global config with path of {}. No such global config present.", path.display()),
                            "Config path error",
                            Span::unknown(),
                        )
                )?
            }
            ConfigPath::Local(path) => {
                configs.local_configs.iter_mut().find(|cfg| &cfg.file_path == path).ok_or_else(||
                        ShellError::labeled_error(
                            &format!("Error reloading local config with path of {}. No such local config present.", path.display()),
                            "Config path error",
                            Span::unknown(),
                        )
                )?
            }
        };

        cfg.reload();

        let exit_scripts = cfg.exit_scripts()?;
        let cfg_paths = cfg.path()?;

        let joined_paths = cfg_paths
            .map(|mut cfg_paths| {
                //existing paths are prepended to path
                if let Some(env_paths) = self.scope.get_env("PATH") {
                    let mut env_paths = std::env::split_paths(&env_paths).collect::<Vec<_>>();
                    //No duplicates! Remove env_paths already existing in cfg_paths
                    env_paths.retain(|env_path| !cfg_paths.contains(env_path));
                    //env_paths entries are appended at the end
                    //nu config paths have a higher priority
                    cfg_paths.extend(env_paths);
                }
                cfg_paths
            })
            .map(|paths| {
                std::env::join_paths(paths)
                    .map(|s| s.to_string_lossy().to_string())
                    .map_err(|e| {
                        ShellError::labeled_error(
                            &format!("Error while joining paths from config: {:?}", e),
                            "Config path error",
                            Span::unknown(),
                        )
                    })
            })
            .transpose()?;

        let tag = config::cfg_path_to_scope_tag(cfg_path);
        let mut frame = ScopeFrame::with_tag(tag.clone());

        frame.env = cfg.env_map();
        if let Some(path) = joined_paths {
            frame.env.insert("PATH".to_string(), path);
        }
        frame.exitscripts = exit_scripts;

        self.scope.update_frame_with_tag(frame, &tag)?;

        Ok(())
    }
