    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: de::Deserializer<'de>
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = StringOrBool;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a boolean or a string")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where E: de::Error,
            {
                Ok(StringOrBool::String(s.to_string()))
            }

            fn visit_bool<E>(self, b: bool) -> Result<Self::Value, E>
                where E: de::Error,
            {
                Ok(StringOrBool::Bool(b))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TomlProject {
    name: String,
    version: semver::Version,
    authors: Option<Vec<String>>,
    build: Option<StringOrBool>,
    links: Option<String>,
    exclude: Option<Vec<String>>,
    include: Option<Vec<String>>,
    publish: Option<bool>,
    workspace: Option<String>,
    #[serde(rename = "im-a-teapot")]
    im_a_teapot: Option<bool>,

    // package metadata
    description: Option<String>,
    homepage: Option<String>,
    documentation: Option<String>,
    readme: Option<String>,
    keywords: Option<Vec<String>>,
    categories: Option<Vec<String>>,
    license: Option<String>,
    #[serde(rename = "license-file")]
    license_file: Option<String>,
    repository: Option<String>,
    metadata: Option<toml::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TomlWorkspace {
    members: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

impl TomlProject {
    pub fn to_package_id(&self, source_id: &SourceId) -> CargoResult<PackageId> {
        PackageId::new(&self.name, self.version.clone(), source_id)
    }
}

struct Context<'a, 'b> {
    pkgid: Option<&'a PackageId>,
    deps: &'a mut Vec<Dependency>,
    source_id: &'a SourceId,
    nested_paths: &'a mut Vec<PathBuf>,
    config: &'b Config,
    warnings: &'a mut Vec<String>,
    platform: Option<Platform>,
    root: &'a Path,
}

impl TomlManifest {
    pub fn prepare_for_publish(&self) -> TomlManifest {
        let mut package = self.package.as_ref()
                              .or_else(|| self.project.as_ref())
                              .unwrap()
                              .clone();
        package.workspace = None;
        return TomlManifest {
            package: Some(package),
            project: None,
            profile: self.profile.clone(),
            lib: self.lib.clone(),
            bin: self.bin.clone(),
            example: self.example.clone(),
            test: self.test.clone(),
            bench: self.bench.clone(),
            dependencies: map_deps(self.dependencies.as_ref()),
            dev_dependencies: map_deps(self.dev_dependencies.as_ref()
                                         .or_else(|| self.dev_dependencies2.as_ref())),
            dev_dependencies2: None,
            build_dependencies: map_deps(self.build_dependencies.as_ref()
                                         .or_else(|| self.build_dependencies2.as_ref())),
            build_dependencies2: None,
            features: self.features.clone(),
            target: self.target.as_ref().map(|target_map| {
                target_map.iter().map(|(k, v)| {
                    (k.clone(), TomlPlatform {
                        dependencies: map_deps(v.dependencies.as_ref()),
                        dev_dependencies: map_deps(v.dev_dependencies.as_ref()
                                                     .or_else(|| v.dev_dependencies2.as_ref())),
                        dev_dependencies2: None,
                        build_dependencies: map_deps(v.build_dependencies.as_ref()
                                                     .or_else(|| v.build_dependencies2.as_ref())),
                        build_dependencies2: None,
                    })
                }).collect()
            }),
            replace: None,
            patch: None,
            workspace: None,
            badges: self.badges.clone(),
            cargo_features: self.cargo_features.clone(),
        };

        fn map_deps(deps: Option<&BTreeMap<String, TomlDependency>>)
                        -> Option<BTreeMap<String, TomlDependency>>
        {
            let deps = match deps {
                Some(deps) => deps,
                None => return None
            };
            Some(deps.iter().map(|(k, v)| (k.clone(), map_dependency(v))).collect())
        }

        fn map_dependency(dep: &TomlDependency) -> TomlDependency {
            match *dep {
                TomlDependency::Detailed(ref d) => {
                    let mut d = d.clone();
                    d.path.take(); // path dependencies become crates.io deps
                    TomlDependency::Detailed(d)
                }
                TomlDependency::Simple(ref s) => {
                    TomlDependency::Detailed(DetailedTomlDependency {
                        version: Some(s.clone()),
                        ..Default::default()
                    })
                }
            }
        }
    }

    fn to_real_manifest(me: &Rc<TomlManifest>,
                        source_id: &SourceId,
                        package_root: &Path,
                        config: &Config)
                        -> CargoResult<(Manifest, Vec<PathBuf>)> {
        let mut nested_paths = vec![];
        let mut warnings = vec![];
        let mut errors = vec![];

        let project = me.project.as_ref().or_else(|| me.package.as_ref());
        let project = project.ok_or_else(|| {
            CargoError::from("no `package` or `project` section found.")
        })?;

        let package_name = project.name.trim();
        if package_name.is_empty() {
            bail!("package name cannot be an empty string.")
        }

        let pkgid = project.to_package_id(source_id)?;

        // If we have no lib at all, use the inferred lib if available
        // If we have a lib with a path, we're done
        // If we have a lib with no path, use the inferred lib or_else package name
        let targets = targets(me, package_name, package_root, &project.build,
                              &mut warnings, &mut errors)?;

        if targets.is_empty() {
            debug!("manifest has no build targets");
        }

        if let Err(e) = unique_build_targets(&targets, package_root) {
            warnings.push(format!("file found to be present in multiple \
                                   build targets: {}", e));
        }

        let mut deps = Vec::new();
        let replace;
        let patch;

        {

            let mut cx = Context {
                pkgid: Some(&pkgid),
                deps: &mut deps,
                source_id: source_id,
                nested_paths: &mut nested_paths,
                config: config,
                warnings: &mut warnings,
                platform: None,
                root: package_root,
            };

            fn process_dependencies(
                cx: &mut Context,
                new_deps: Option<&BTreeMap<String, TomlDependency>>,
                kind: Option<Kind>)
                -> CargoResult<()>
            {
                let dependencies = match new_deps {
                    Some(dependencies) => dependencies,
                    None => return Ok(())
                };
                for (n, v) in dependencies.iter() {
                    let dep = v.to_dependency(n, cx, kind)?;
                    cx.deps.push(dep);
                }

                Ok(())
            }

            // Collect the deps
            process_dependencies(&mut cx, me.dependencies.as_ref(),
                                 None)?;
            let dev_deps = me.dev_dependencies.as_ref()
                               .or_else(|| me.dev_dependencies2.as_ref());
            process_dependencies(&mut cx, dev_deps, Some(Kind::Development))?;
            let build_deps = me.build_dependencies.as_ref()
                               .or_else(|| me.build_dependencies2.as_ref());
            process_dependencies(&mut cx, build_deps, Some(Kind::Build))?;

            for (name, platform) in me.target.iter().flat_map(|t| t) {
                cx.platform = Some(name.parse()?);
                process_dependencies(&mut cx, platform.dependencies.as_ref(),
                                     None)?;
                let build_deps = platform.build_dependencies.as_ref()
                                         .or_else(|| platform.build_dependencies2.as_ref());
                process_dependencies(&mut cx, build_deps, Some(Kind::Build))?;
                let dev_deps = platform.dev_dependencies.as_ref()
                                         .or_else(|| platform.dev_dependencies2.as_ref());
                process_dependencies(&mut cx, dev_deps, Some(Kind::Development))?;
            }

            replace = me.replace(&mut cx)?;
            patch = me.patch(&mut cx)?;
        }

        {
            let mut names_sources = BTreeMap::new();
            for dep in &deps {
                let name = dep.name();
                let prev = names_sources.insert(name, dep.source_id());
                if prev.is_some() && prev != Some(dep.source_id()) {
                    bail!("Dependency '{}' has different source paths depending on the build \
                           target. Each dependency must have a single canonical source path \
                           irrespective of build target.", name);
                }
            }
        }

        let exclude = project.exclude.clone().unwrap_or_default();
        let include = project.include.clone().unwrap_or_default();

        let summary = Summary::new(pkgid, deps, me.features.clone()
            .unwrap_or_else(BTreeMap::new))?;
        let metadata = ManifestMetadata {
            description: project.description.clone(),
            homepage: project.homepage.clone(),
            documentation: project.documentation.clone(),
            readme: project.readme.clone(),
            authors: project.authors.clone().unwrap_or_default(),
            license: project.license.clone(),
            license_file: project.license_file.clone(),
            repository: project.repository.clone(),
            keywords: project.keywords.clone().unwrap_or_default(),
            categories: project.categories.clone().unwrap_or_default(),
            badges: me.badges.clone().unwrap_or_default(),
        };

        let workspace_config = match (me.workspace.as_ref(),
                                      project.workspace.as_ref()) {
            (Some(config), None) => {
                WorkspaceConfig::Root {
                    members: config.members.clone(),
                    exclude: config.exclude.clone().unwrap_or_default(),
                }
            }
            (None, root) => {
                WorkspaceConfig::Member { root: root.cloned() }
            }
            (Some(..), Some(..)) => {
                bail!("cannot configure both `package.workspace` and \
                       `[workspace]`, only one can be specified")
            }
        };
        let profiles = build_profiles(&me.profile);
        let publish = project.publish.unwrap_or(true);
        let empty = Vec::new();
        let cargo_features = me.cargo_features.as_ref().unwrap_or(&empty);
        let features = Features::new(cargo_features, &mut warnings)?;
        let mut manifest = Manifest::new(summary,
                                         targets,
                                         exclude,
                                         include,
                                         project.links.clone(),
                                         metadata,
                                         profiles,
                                         publish,
                                         replace,
                                         patch,
                                         workspace_config,
                                         features,
                                         project.im_a_teapot,
                                         Rc::clone(me));
        if project.license_file.is_some() && project.license.is_some() {
            manifest.add_warning("only one of `license` or \
                                 `license-file` is necessary".to_string());
        }
        for warning in warnings {
            manifest.add_warning(warning);
        }
        for error in errors {
            manifest.add_critical_warning(error);
        }

        manifest.feature_gate()?;

        Ok((manifest, nested_paths))
    }

    fn to_virtual_manifest(me: &Rc<TomlManifest>,
                           source_id: &SourceId,
                           root: &Path,
                           config: &Config)
                           -> CargoResult<(VirtualManifest, Vec<PathBuf>)> {
        if me.project.is_some() {
            bail!("virtual manifests do not define [project]");
        }
        if me.package.is_some() {
            bail!("virtual manifests do not define [package]");
        }
        if me.lib.is_some() {
            bail!("virtual manifests do not specify [lib]");
        }
        if me.bin.is_some() {
            bail!("virtual manifests do not specify [[bin]]");
        }
        if me.example.is_some() {
            bail!("virtual manifests do not specify [[example]]");
        }
        if me.test.is_some() {
            bail!("virtual manifests do not specify [[test]]");
        }
        if me.bench.is_some() {
            bail!("virtual manifests do not specify [[bench]]");
        }

        let mut nested_paths = Vec::new();
        let mut warnings = Vec::new();
        let mut deps = Vec::new();
        let (replace, patch) = {
            let mut cx = Context {
                pkgid: None,
                deps: &mut deps,
                source_id: source_id,
                nested_paths: &mut nested_paths,
                config: config,
                warnings: &mut warnings,
                platform: None,
                root: root
            };
            (me.replace(&mut cx)?, me.patch(&mut cx)?)
        };
        let profiles = build_profiles(&me.profile);
        let workspace_config = match me.workspace {
            Some(ref config) => {
                WorkspaceConfig::Root {
                    members: config.members.clone(),
                    exclude: config.exclude.clone().unwrap_or_default(),
                }
            }
            None => {
                bail!("virtual manifests must be configured with [workspace]");
            }
        };
        Ok((VirtualManifest::new(replace, patch, workspace_config, profiles), nested_paths))
    }

    fn replace(&self, cx: &mut Context)
               -> CargoResult<Vec<(PackageIdSpec, Dependency)>> {
        if self.patch.is_some() && self.replace.is_some() {
            bail!("cannot specify both [replace] and [patch]");
        }
        let mut replace = Vec::new();
        for (spec, replacement) in self.replace.iter().flat_map(|x| x) {
            let mut spec = PackageIdSpec::parse(spec).chain_err(|| {
                format!("replacements must specify a valid semver \
                         version to replace, but `{}` does not",
                        spec)
            })?;
            if spec.url().is_none() {
                spec.set_url(CRATES_IO.parse().unwrap());
            }

            let version_specified = match *replacement {
                TomlDependency::Detailed(ref d) => d.version.is_some(),
                TomlDependency::Simple(..) => true,
            };
            if version_specified {
                bail!("replacements cannot specify a version \
                       requirement, but found one for `{}`", spec);
            }

            let mut dep = replacement.to_dependency(spec.name(), cx, None)?;
            {
                let version = spec.version().ok_or_else(|| {
                    CargoError::from(format!("replacements must specify a version \
                             to replace, but `{}` does not",
                            spec))
                })?;
                dep.set_version_req(VersionReq::exact(version));
            }
            replace.push((spec, dep));
        }
        Ok(replace)
    }

    fn patch(&self, cx: &mut Context)
             -> CargoResult<HashMap<Url, Vec<Dependency>>> {
        let mut patch = HashMap::new();
        for (url, deps) in self.patch.iter().flat_map(|x| x) {
            let url = match &url[..] {
                "crates-io" => CRATES_IO.parse().unwrap(),
                _ => url.to_url()?,
            };
            patch.insert(url, deps.iter().map(|(name, dep)| {
                dep.to_dependency(name, cx, None)
            }).collect::<CargoResult<Vec<_>>>()?);
        }
        Ok(patch)
    }

    fn maybe_custom_build(&self,
                          build: &Option<StringOrBool>,
                          package_root: &Path)
                          -> Option<PathBuf> {
        let build_rs = package_root.join("build.rs");
        match *build {
            Some(StringOrBool::Bool(false)) => None,        // explicitly no build script
            Some(StringOrBool::Bool(true)) => Some(build_rs.into()),
            Some(StringOrBool::String(ref s)) => Some(PathBuf::from(s)),
            None => {
                match fs::metadata(&build_rs) {
                    // If there is a build.rs file next to the Cargo.toml, assume it is
                    // a build script
                    Ok(ref e) if e.is_file() => Some(build_rs.into()),
                    Ok(_) | Err(_) => None,
                }
            }
        }
    }
}
