    pub fn new(config: &'a Config, mode: CompileMode) -> CargoResult<CompileOptions<'a>> {
        Ok(CompileOptions {
            config,
            build_config: BuildConfig::new(config, None, &None, mode)?,
            features: Vec::new(),
            all_features: false,
            no_default_features: false,
            spec: ops::Packages::Packages(Vec::new()),
            filter: CompileFilter::Default {
                required_features_filterable: false,
            },
            target_rustdoc_args: None,
            target_rustc_args: None,
            export_dir: None,
        })
    }

    // Returns the unique specified package, or None
    pub fn get_package<'b>(&self, ws: &'b Workspace) -> CargoResult<Option<&'b Package>> {
        Ok(match self.spec {
            Packages::All | Packages::Default | Packages::OptOut(_) => {
                None
            }
            Packages::Packages(ref xs) => match xs.len() {
                0 => Some(ws.current()?),
                1 => Some(ws.members()
                    .find(|pkg| *pkg.name() == xs[0])
                    .ok_or_else(|| {
                        format_err!("package `{}` is not a member of the workspace", xs[0])
                    })?),
                _ => None,
            },
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Packages {
    Default,
    All,
    OptOut(Vec<String>),
    Packages(Vec<String>),
}

impl Packages {
    pub fn from_flags(all: bool, exclude: Vec<String>, package: Vec<String>) -> CargoResult<Self> {
        Ok(match (all, exclude.len(), package.len()) {
            (false, 0, 0) => Packages::Default,
            (false, 0, _) => Packages::Packages(package),
            (false, _, _) => bail!("--exclude can only be used together with --all"),
            (true, 0, _) => Packages::All,
            (true, _, _) => Packages::OptOut(exclude),
        })
    }

    pub fn to_package_id_specs(&self, ws: &Workspace) -> CargoResult<Vec<PackageIdSpec>> {
        let specs = match *self {
            Packages::All => ws.members()
                .map(Package::package_id)
                .map(PackageIdSpec::from_package_id)
                .collect(),
            Packages::OptOut(ref opt_out) => ws.members()
                .map(Package::package_id)
                .map(PackageIdSpec::from_package_id)
                .filter(|p| opt_out.iter().position(|x| *x == p.name()).is_none())
                .collect(),
            Packages::Packages(ref packages) if packages.is_empty() => {
                vec![PackageIdSpec::from_package_id(ws.current()?.package_id())]
            }
            Packages::Packages(ref packages) => packages
                .iter()
                .map(|p| PackageIdSpec::parse(p))
                .collect::<CargoResult<Vec<_>>>()?,
            Packages::Default => ws.default_members()
                .map(Package::package_id)
                .map(PackageIdSpec::from_package_id)
                .collect(),
        };
        if specs.is_empty() {
            if ws.is_virtual() {
                bail!(
                    "manifest path `{}` contains no package: The manifest is virtual, \
                     and the workspace has no members.",
                    ws.root().display()
                )
            }
            bail!("no packages to compile")
        }
        Ok(specs)
    }
}

#[derive(Debug)]
pub enum FilterRule {
    All,
    Just(Vec<String>),
}

#[derive(Debug)]
pub enum CompileFilter {
    Default {
        /// Flag whether targets can be safely skipped when required-features are not satisfied.
        required_features_filterable: bool,
    },
    Only {
        all_targets: bool,
        lib: bool,
        bins: FilterRule,
        examples: FilterRule,
        tests: FilterRule,
        benches: FilterRule,
    },
}

pub fn compile<'a>(
    ws: &Workspace<'a>,
    options: &CompileOptions<'a>,
) -> CargoResult<Compilation<'a>> {
    let exec: Arc<Executor> = Arc::new(DefaultExecutor);
    compile_with_exec(ws, options, &exec)
}

/// Like `compile` but allows specifying a custom `Executor` that will be able to intercept build
/// calls and add custom logic. `compile` uses `DefaultExecutor` which just passes calls through.
pub fn compile_with_exec<'a>(
    ws: &Workspace<'a>,
    options: &CompileOptions<'a>,
    exec: &Arc<Executor>,
) -> CargoResult<Compilation<'a>> {
    ws.emit_warnings()?;
    compile_ws(ws, None, options, exec)
}

pub fn compile_ws<'a>(
    ws: &Workspace<'a>,
    source: Option<Box<Source + 'a>>,
    options: &CompileOptions<'a>,
    exec: &Arc<Executor>,
) -> CargoResult<Compilation<'a>> {
    let CompileOptions {
        config,
        ref build_config,
        ref spec,
        ref features,
        all_features,
        no_default_features,
        ref filter,
        ref target_rustdoc_args,
        ref target_rustc_args,
        ref export_dir,
    } = *options;

    let default_arch_kind = if build_config.requested_target.is_some() {
        Kind::Target
    } else {
        Kind::Host
    };

    let specs = spec.to_package_id_specs(ws)?;
    let features = Method::split_features(features);
    let method = Method::Required {
        dev_deps: ws.require_optional_deps() || filter.need_dev_deps(build_config.mode),
        features: &features,
        all_features,
        uses_default_features: !no_default_features,
    };
    let resolve = ops::resolve_ws_with_method(ws, source, method, &specs)?;
    let (packages, resolve_with_overrides) = resolve;

    let to_builds = specs
        .iter()
        .map(|p| {
            let pkgid = p.query(resolve_with_overrides.iter())?;
            let p = packages.get(pkgid)?;
            p.manifest().print_teapot(ws.config());
            Ok(p)
        })
        .collect::<CargoResult<Vec<_>>>()?;

    let (extra_args, extra_args_name) = match (target_rustc_args, target_rustdoc_args) {
        (&Some(ref args), _) => (Some(args.clone()), "rustc"),
        (_, &Some(ref args)) => (Some(args.clone()), "rustdoc"),
        _ => (None, ""),
    };

    if extra_args.is_some() && to_builds.len() != 1 {
        panic!(
            "`{}` should not accept multiple `-p` flags",
            extra_args_name
        );
    }

    let profiles = ws.profiles();
    profiles.validate_packages(&mut config.shell(), &packages)?;

    let mut extra_compiler_args = None;

    let units = generate_targets(
        ws,
        profiles,
        &to_builds,
        filter,
        default_arch_kind,
        &resolve_with_overrides,
        build_config,
    )?;

    if let Some(args) = extra_args {
        if units.len() != 1 {
            bail!(
                "extra arguments to `{}` can only be passed to one \
                 target, consider filtering\nthe package by passing \
                 e.g. `--lib` or `--bin NAME` to specify a single target",
                extra_args_name
            );
        }
        extra_compiler_args = Some((units[0], args));
    }

    let ret = {
        let _p = profile::start("compiling");
        let bcx = BuildContext::new(
            ws,
            &resolve_with_overrides,
            &packages,
            config,
            &build_config,
            profiles,
            extra_compiler_args,
        )?;
        let mut cx = Context::new(config, &bcx)?;
        cx.compile(&units, export_dir.clone(), &exec)?
    };

    Ok(ret)
}
