pub fn prepare_target<'a, 'cfg>(cx: &mut Context<'a, 'cfg>,
                                unit: &Unit<'a>) -> CargoResult<Preparation> {
    let _p = profile::start(format!("fingerprint: {} / {}",
                                    unit.pkg.package_id(), unit.target.name()));
    let new = dir(cx, unit);
    let loc = new.join(&filename(unit));

    debug!("fingerprint at: {}", loc.display());

    let fingerprint = try!(calculate(cx, unit));
    let compare = compare_old_fingerprint(&loc, &*fingerprint);
    log_compare(unit, &compare);

    let root = cx.out_dir(unit);
    let mut missing_outputs = false;
    if unit.profile.doc {
        missing_outputs = !root.join(unit.target.crate_name())
                               .join("index.html").exists();
    } else {
        for (filename, _) in try!(cx.target_filenames(unit)) {
            missing_outputs |= fs::metadata(root.join(filename)).is_err();
        }
    }

    let allow_failure = unit.profile.rustc_args.is_some();
    let write_fingerprint = Work::new(move |_| {
        match fingerprint.update_local() {
            Ok(()) => {}
            Err(..) if allow_failure => return Ok(()),
            Err(e) => return Err(e)
        }
        write_fingerprint(&loc, &*fingerprint)
    });

    let fresh = compare.is_ok() && !missing_outputs;
    Ok((if fresh {Fresh} else {Dirty}, write_fingerprint, Work::noop()))
}

/// A fingerprint can be considered to be a "short string" representing the
/// state of a world for a package.
///
/// If a fingerprint ever changes, then the package itself needs to be
/// recompiled. Inputs to the fingerprint include source code modifications,
/// compiler flags, compiler version, etc. This structure is not simply a
/// `String` due to the fact that some fingerprints cannot be calculated lazily.
///
/// Path sources, for example, use the mtime of the corresponding dep-info file
/// as a fingerprint (all source files must be modified *before* this mtime).
/// This dep-info file is not generated, however, until after the crate is
/// compiled. As a result, this structure can be thought of as a fingerprint
/// to-be. The actual value can be calculated via `hash()`, but the operation
/// may fail as some files may not have been generated.
///
/// Note that dependencies are taken into account for fingerprints because rustc
/// requires that whenever an upstream crate is recompiled that all downstream
/// dependants are also recompiled. This is typically tracked through
/// `DependencyQueue`, but it also needs to be retained here because Cargo can
/// be interrupted while executing, losing the state of the `DependencyQueue`
/// graph.
pub struct Fingerprint {
    rustc: u64,
    features: String,
    target: u64,
    profile: u64,
    deps: Vec<(String, Arc<Fingerprint>)>,
    local: LocalFingerprint,
    memoized_hash: Mutex<Option<u64>>,
    rustflags: Vec<String>,
}

#[derive(RustcEncodable, RustcDecodable, Hash)]
enum LocalFingerprint {
    Precalculated(String),
    MtimeBased(MtimeSlot, PathBuf),
}

struct MtimeSlot(Mutex<Option<FileTime>>);

impl Fingerprint {
    fn update_local(&self) -> CargoResult<()> {
        match self.local {
            LocalFingerprint::MtimeBased(ref slot, ref path) => {
                let meta = try!(fs::metadata(path).chain_error(|| {
                    internal(format!("failed to stat `{}`", path.display()))
                }));
                let mtime = FileTime::from_last_modification_time(&meta);
                *slot.0.lock().unwrap() = Some(mtime);
            }
            LocalFingerprint::Precalculated(..) => return Ok(())
        }

        *self.memoized_hash.lock().unwrap() = None;
        Ok(())
    }

    fn hash(&self) -> u64 {
        if let Some(s) = *self.memoized_hash.lock().unwrap() {
            return s
        }
        let ret = util::hash_u64(self);
        *self.memoized_hash.lock().unwrap() = Some(ret);
        ret
    }

    fn compare(&self, old: &Fingerprint) -> CargoResult<()> {
        if self.rustc != old.rustc {
            bail!("rust compiler has changed")
        }
        if self.features != old.features {
            bail!("features have changed: {} != {}", self.features, old.features)
        }
        if self.target != old.target {
            bail!("target configuration has changed")
        }
        if self.profile != old.profile {
            bail!("profile configuration has changed")
        }
        if self.rustflags != old.rustflags {
            return Err(internal("RUSTFLAGS has changed"))
        }
        match (&self.local, &old.local) {
            (&LocalFingerprint::Precalculated(ref a),
             &LocalFingerprint::Precalculated(ref b)) => {
                if a != b {
                    bail!("precalculated components have changed: {} != {}",
                          a, b)
                }
            }
            (&LocalFingerprint::MtimeBased(ref a, ref ap),
             &LocalFingerprint::MtimeBased(ref b, ref bp)) => {
                let a = a.0.lock().unwrap();
                let b = b.0.lock().unwrap();
                if *a != *b {
                    bail!("mtime based components have changed: {:?} != {:?}, \
                           paths are {:?} and {:?}", *a, *b, ap, bp)
                }
            }
            _ => bail!("local fingerprint type has changed"),
        }

        if self.deps.len() != old.deps.len() {
            bail!("number of dependencies has changed")
        }
        for (a, b) in self.deps.iter().zip(old.deps.iter()) {
            if a.1.hash() != b.1.hash() {
                bail!("new ({}) != old ({})", a.0, b.0)
            }
        }
        Ok(())
    }
}

impl hash::Hash for Fingerprint {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let Fingerprint {
            rustc,
            ref features,
            target,
            profile,
            ref deps,
            ref local,
            memoized_hash: _,
            ref rustflags,
        } = *self;
        (rustc, features, target, profile, deps, local, rustflags).hash(h)
    }
}

impl Encodable for Fingerprint {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
        e.emit_struct("Fingerprint", 6, |e| {
            try!(e.emit_struct_field("rustc", 0, |e| self.rustc.encode(e)));
            try!(e.emit_struct_field("target", 1, |e| self.target.encode(e)));
            try!(e.emit_struct_field("profile", 2, |e| self.profile.encode(e)));
            try!(e.emit_struct_field("local", 3, |e| self.local.encode(e)));
            try!(e.emit_struct_field("features", 4, |e| {
                self.features.encode(e)
            }));
            try!(e.emit_struct_field("deps", 5, |e| {
                self.deps.iter().map(|&(ref a, ref b)| {
                    (a, b.hash())
                }).collect::<Vec<_>>().encode(e)
            }));
            try!(e.emit_struct_field("rustflags", 6, |e| self.rustflags.encode(e)));
            Ok(())
        })
    }
}

impl Decodable for Fingerprint {
    fn decode<D: Decoder>(d: &mut D) -> Result<Fingerprint, D::Error> {
        fn decode<T: Decodable, D: Decoder>(d: &mut D) -> Result<T, D::Error> {
            Decodable::decode(d)
        }
        d.read_struct("Fingerprint", 6, |d| {
            Ok(Fingerprint {
                rustc: try!(d.read_struct_field("rustc", 0, decode)),
                target: try!(d.read_struct_field("target", 1, decode)),
                profile: try!(d.read_struct_field("profile", 2, decode)),
                local: try!(d.read_struct_field("local", 3, decode)),
                features: try!(d.read_struct_field("features", 4, decode)),
                memoized_hash: Mutex::new(None),
                deps: {
                    let decode = decode::<Vec<(String, u64)>, D>;
                    let v = try!(d.read_struct_field("deps", 5, decode));
                    v.into_iter().map(|(name, hash)| {
                        (name, Arc::new(Fingerprint {
                            rustc: 0,
                            target: 0,
                            profile: 0,
                            local: LocalFingerprint::Precalculated(String::new()),
                            features: String::new(),
                            deps: Vec::new(),
                            memoized_hash: Mutex::new(Some(hash)),
                            rustflags: Vec::new(),
                        }))
                    }).collect()
                },
                rustflags: try!(d.read_struct_field("rustflags", 6, decode)),
            })
        })
    }
}

impl hash::Hash for MtimeSlot {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.0.lock().unwrap().hash(h)
    }
}

impl Encodable for MtimeSlot {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
        self.0.lock().unwrap().map(|ft| {
            (ft.seconds_relative_to_1970(), ft.nanoseconds())
        }).encode(e)
    }
}

impl Decodable for MtimeSlot {
    fn decode<D: Decoder>(e: &mut D) -> Result<MtimeSlot, D::Error> {
        let kind: Option<(u64, u32)> = try!(Decodable::decode(e));
        Ok(MtimeSlot(Mutex::new(kind.map(|(s, n)| {
            FileTime::from_seconds_since_1970(s, n)
        }))))
    }
}

/// Calculates the fingerprint for a package/target pair.
///
/// This fingerprint is used by Cargo to learn about when information such as:
///
/// * A non-path package changes (changes version, changes revision, etc).
/// * Any dependency changes
/// * The compiler changes
/// * The set of features a package is built with changes
/// * The profile a target is compiled with changes (e.g. opt-level changes)
///
/// Information like file modification time is only calculated for path
/// dependencies and is calculated in `calculate_target_fresh`.
fn calculate<'a, 'cfg>(cx: &mut Context<'a, 'cfg>, unit: &Unit<'a>)
                       -> CargoResult<Arc<Fingerprint>> {
    if let Some(s) = cx.fingerprints.get(unit) {
        return Ok(s.clone())
    }

    // First, calculate all statically known "salt data" such as the profile
    // information (compiler flags), the compiler version, activated features,
    // and target configuration.
    let features = cx.resolve.features(unit.pkg.package_id());
    let features = features.map(|s| {
        let mut v = s.iter().collect::<Vec<_>>();
        v.sort();
        v
    });

    // Next, recursively calculate the fingerprint for all of our dependencies.
    //
    // Skip the fingerprints of build scripts as they may not always be
    // available and the dirtiness propagation for modification is tracked
    // elsewhere. Also skip fingerprints of binaries because they don't actually
    // induce a recompile, they're just dependencies in the sense that they need
    // to be built.
    let deps = try!(cx.dep_targets(unit));
    let deps = try!(deps.iter().filter(|u| {
        !u.target.is_custom_build() && !u.target.is_bin()
    }).map(|unit| {
        calculate(cx, unit).map(|fingerprint| {
            (unit.pkg.package_id().to_string(), fingerprint)
        })
    }).collect::<CargoResult<Vec<_>>>());

    // And finally, calculate what our own local fingerprint is
    let local = if use_dep_info(unit) {
        let dep_info = dep_info_loc(cx, unit);
        let mtime = try!(dep_info_mtime_if_fresh(&dep_info));
        LocalFingerprint::MtimeBased(MtimeSlot(Mutex::new(mtime)), dep_info)
    } else {
        let fingerprint = try!(pkg_fingerprint(cx, unit.pkg));
        LocalFingerprint::Precalculated(fingerprint)
    };
    let mut deps = deps;
    deps.sort_by(|&(ref a, _), &(ref b, _)| a.cmp(b));
    let extra_flags = if unit.profile.doc {
        try!(cx.rustdocflags_args(unit))
    } else {
        try!(cx.rustflags_args(unit))
    };
    let fingerprint = Arc::new(Fingerprint {
        rustc: util::hash_u64(&cx.config.rustc_info().verbose_version),
        target: util::hash_u64(&unit.target),
        profile: util::hash_u64(&unit.profile),
        features: format!("{:?}", features),
        deps: deps,
        local: local,
        memoized_hash: Mutex::new(None),
        rustflags: extra_flags,
    });
    cx.fingerprints.insert(*unit, fingerprint.clone());
    Ok(fingerprint)
}
