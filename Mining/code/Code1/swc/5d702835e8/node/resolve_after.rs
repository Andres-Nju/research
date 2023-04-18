    fn resolve(&self, base: &FileName, target: &str) -> Result<FileName, Error> {
        log::debug!(
            "Resolve {} from {:#?} for {:#?}",
            target,
            base,
            self.target_env
        );

        let base = match base {
            FileName::Real(v) => v,
            _ => bail!("node-resolver supports only files"),
        };

        let cwd = &Path::new(".");
        let base_dir = base.parent().unwrap_or(&cwd);

        // Handle module references for the `browser` package config
        // before we map aliases.
        if let TargetEnv::Browser = self.target_env {
            if let Some(pkg_base) = find_package_root(base) {
                if let Some(item) = BROWSER_CACHE.get(&pkg_base) {
                    let value = item.value();
                    if value.module_ignores.contains(target) {
                        return Ok(FileName::Custom(target.into()));
                    }
                    if let Some(rewrite) = value.module_rewrites.get(target) {
                        return self.wrap(Some(rewrite.to_path_buf()));
                    }
                }
            }
        }

        // Handle builtin modules for nodejs
        if let TargetEnv::Node = self.target_env {
            if is_core_module(target) {
                return Ok(FileName::Custom(format!("node:{}", target.to_string())));
            }
        }

        // Aliases allow browser shims to be renamed so we can
        // map `stream` to `stream-browserify` for example
        let target = if let Some(alias) = self.alias.get(target) {
            &alias[..]
        } else {
            target
        };

        let target_path = Path::new(target);

        let file_name = {
            if target_path.is_absolute() {
                let path = PathBuf::from(target_path);
                self.resolve_as_file(&path)
                    .or_else(|_| self.resolve_as_directory(&path))
                    .and_then(|p| self.wrap(p))
            } else {
                let mut components = target_path.components();

                if let Some(Component::CurDir | Component::ParentDir) = components.next() {
                    #[cfg(windows)]
                    let path = {
                        let base_dir = BasePath::new(base_dir).unwrap();
                        base_dir
                            .join(target.replace('/', "\\"))
                            .normalize_virtually()
                            .unwrap()
                            .into_path_buf()
                    };
                    #[cfg(not(windows))]
                    let path = base_dir.join(target);
                    self.resolve_as_file(&path)
                        .or_else(|_| self.resolve_as_directory(&path))
                        .and_then(|p| self.wrap(p))
                } else {
                    self.resolve_node_modules(base, target)
                        .and_then(|p| self.wrap(p))
                }
            }
        }
        .and_then(|v| {
            // Handle path references for the `browser` package config
            if let TargetEnv::Browser = self.target_env {
                if let FileName::Real(path) = &v {
                    if let Some(pkg_base) = find_package_root(base) {
                        if let Some(item) = BROWSER_CACHE.get(&pkg_base) {
                            let value = item.value();
                            if value.ignores.contains(path) {
                                return Ok(FileName::Custom(path.display().to_string().into()));
                            }
                            if let Some(rewrite) = value.rewrites.get(path) {
                                return self.wrap(Some(rewrite.to_path_buf()));
                            }
                        }
                    }
                }
            }
            Ok(v)
        });

        file_name
    }
