use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::{self, Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use flate2::read::GzDecoder;
use flate2::{Compression, GzBuilder};
use log::debug;
use serde_json::{self, json};
use tar::{Archive, Builder, EntryType, Header};
use termcolor::Color;

use crate::core::compiler::{BuildConfig, CompileMode, DefaultExecutor, Executor};
use crate::core::resolver::Method;
use crate::core::Feature;
use crate::core::{
    Package, PackageId, PackageIdSpec, PackageSet, Resolve, Source, SourceId, Verbosity, Workspace,
};
use crate::ops;
use crate::sources::PathSource;
use crate::util::errors::{CargoResult, CargoResultExt};
use crate::util::paths;
use crate::util::toml::TomlManifest;
use crate::util::{self, internal, Config, FileLock};

pub struct PackageOpts<'cfg> {
    pub config: &'cfg Config,
    pub list: bool,
    pub check_metadata: bool,
    pub allow_dirty: bool,
    pub verify: bool,
    pub jobs: Option<u32>,
    pub target: Option<String>,
    pub features: Vec<String>,
    pub all_features: bool,
    pub no_default_features: bool,
}

static VCS_INFO_FILE: &'static str = ".cargo_vcs_info.json";

pub fn package(ws: &Workspace<'_>, opts: &PackageOpts<'_>) -> CargoResult<Option<FileLock>> {
    // Make sure the Cargo.lock is up-to-date and valid.
    ops::resolve_ws(ws)?;
    let pkg = ws.current()?;
    let config = ws.config();

    let mut src = PathSource::new(pkg.root(), pkg.package_id().source_id(), config);
    src.update()?;

    if opts.check_metadata {
        check_metadata(pkg, config)?;
    }

    verify_dependencies(pkg)?;

    if !pkg.manifest().exclude().is_empty() && !pkg.manifest().include().is_empty() {
        config.shell().warn(
            "both package.include and package.exclude are specified; \
             the exclude list will be ignored",
        )?;
    }
    // `list_files` outputs warnings as a side effect, so only do it once.
    let src_files = src.list_files(pkg)?;

    // Make sure a VCS info file is not included in source, regardless of if
    // we produced the file above, and in particular if we did not.
    check_vcs_file_collision(pkg, &src_files)?;

    // Check (git) repository state, getting the current commit hash if not
    // dirty. This will `bail!` if dirty, unless allow_dirty. Produce json
    // info for any sha1 (HEAD revision) returned.
    let vcs_info = if !opts.allow_dirty {
        check_repo_state(pkg, &src_files, config, opts.allow_dirty)?
            .map(|h| json!({"git":{"sha1": h}}))
    } else {
        None
    };

    if opts.list {
        let root = pkg.root();
        let mut list: Vec<_> = src
            .list_files(pkg)?
            .iter()
            .map(|file| file.strip_prefix(root).unwrap().to_path_buf())
            .collect();
        if pkg.include_lockfile() && !list.contains(&PathBuf::from("Cargo.lock")) {
            // A generated Cargo.lock will be included.
            list.push("Cargo.lock".into());
        }
        if vcs_info.is_some() {
            list.push(Path::new(VCS_INFO_FILE).to_path_buf());
        }
        list.sort_unstable();
        for file in list.iter() {
            println!("{}", file.display());
        }
        return Ok(None);
    }

    let filename = format!("{}-{}.crate", pkg.name(), pkg.version());
    let dir = ws.target_dir().join("package");
    let mut dst = {
        let tmp = format!(".{}", filename);
        dir.open_rw(&tmp, config, "package scratch space")?
    };

    // Package up and test a temporary tarball and only move it to the final
    // location if it actually passes all our tests. Any previously existing
    // tarball can be assumed as corrupt or invalid, so we just blow it away if
    // it exists.
    config
        .shell()
        .status("Packaging", pkg.package_id().to_string())?;
    dst.file().set_len(0)?;
    tar(ws, &src_files, vcs_info.as_ref(), dst.file(), &filename)
        .chain_err(|| failure::format_err!("failed to prepare local package for uploading"))?;
    if opts.verify {
        dst.seek(SeekFrom::Start(0))?;
        run_verify(ws, &dst, opts).chain_err(|| "failed to verify package tarball")?
    }
    dst.seek(SeekFrom::Start(0))?;
    {
        let src_path = dst.path();
        let dst_path = dst.parent().join(&filename);
        fs::rename(&src_path, &dst_path)
            .chain_err(|| "failed to move temporary tarball into final location")?;
    }
    Ok(Some(dst))
}

/// Construct `Cargo.lock` for the package to be published.
fn build_lock(ws: &Workspace<'_>) -> CargoResult<String> {
    let config = ws.config();
    let orig_resolve = ops::load_pkg_lockfile(ws)?;

    // Convert Package -> TomlManifest -> Manifest -> Package
    let orig_pkg = ws.current()?;
    let toml_manifest = Rc::new(orig_pkg.manifest().original().prepare_for_publish(config)?);
    let package_root = orig_pkg.root();
    let source_id = orig_pkg.package_id().source_id();
    let (manifest, _nested_paths) =
        TomlManifest::to_real_manifest(&toml_manifest, source_id, package_root, config)?;
    let new_pkg = Package::new(manifest, orig_pkg.manifest_path());

    // Regenerate Cargo.lock using the old one as a guide.
    let specs = vec![PackageIdSpec::from_package_id(new_pkg.package_id())];
    let tmp_ws = Workspace::ephemeral(new_pkg, ws.config(), None, true)?;
    let (pkg_set, new_resolve) = ops::resolve_ws_with_method(&tmp_ws, Method::Everything, &specs)?;

    if let Some(orig_resolve) = orig_resolve {
        compare_resolve(config, tmp_ws.current()?, &orig_resolve, &new_resolve)?;
    }
    check_yanked(config, &pkg_set, &new_resolve)?;

    ops::resolve_to_string(&tmp_ws, &new_resolve)
}

// Checks that the package has some piece of metadata that a human can
// use to tell what the package is about.
fn check_metadata(pkg: &Package, config: &Config) -> CargoResult<()> {
    let md = pkg.manifest().metadata();

    let mut missing = vec![];

    macro_rules! lacking {
        ($( $($field: ident)||* ),*) => {{
            $(
                if $(md.$field.as_ref().map_or(true, |s| s.is_empty()))&&* {
                    $(missing.push(stringify!($field).replace("_", "-"));)*
                }
            )*
        }}
    }
    lacking!(
        description,
        license || license_file,
        documentation || homepage || repository
    );

    if !missing.is_empty() {
        let mut things = missing[..missing.len() - 1].join(", ");
        // `things` will be empty if and only if its length is 1 (i.e., the only case
        // to have no `or`).
        if !things.is_empty() {
            things.push_str(" or ");
        }
        things.push_str(missing.last().unwrap());

        config.shell().warn(&format!(
            "manifest has no {things}.\n\
             See https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata for more info.",
            things = things
        ))?
    }
    Ok(())
}

// Checks that the package dependencies are safe to deploy.
fn verify_dependencies(pkg: &Package) -> CargoResult<()> {
    for dep in pkg.dependencies() {
        if dep.source_id().is_path() && !dep.specified_req() {
            failure::bail!(
                "all path dependencies must have a version specified \
                 when packaging.\ndependency `{}` does not specify \
                 a version.",
                dep.name_in_toml()
            )
        }
    }
    Ok(())
}

// Checks if the package source is in a *git* DVCS repository. If *git*, and
// the source is *dirty* (e.g., has uncommitted changes) and not `allow_dirty`
// then `bail!` with an informative message. Otherwise return the sha1 hash of
// the current *HEAD* commit, or `None` if *dirty*.
fn check_repo_state(
    p: &Package,
    src_files: &[PathBuf],
    config: &Config,
    allow_dirty: bool,
) -> CargoResult<Option<String>> {
    if let Ok(repo) = git2::Repository::discover(p.root()) {
        if let Some(workdir) = repo.workdir() {
            debug!("found a git repo at {:?}", workdir);
            let path = p.manifest_path();
            let path = path.strip_prefix(workdir).unwrap_or(path);
            if let Ok(status) = repo.status_file(path) {
                if (status & git2::Status::IGNORED).is_empty() {
                    debug!(
                        "found (git) Cargo.toml at {:?} in workdir {:?}",
                        path, workdir
                    );
                    return git(p, src_files, &repo, allow_dirty);
                }
            }
            config.shell().verbose(|shell| {
                shell.warn(format!(
                    "No (git) Cargo.toml found at `{}` in workdir `{}`",
                    path.display(),
                    workdir.display()
                ))
            })?;
        }
    } else {
        config.shell().verbose(|shell| {
            shell.warn(format!("No (git) VCS found for `{}`", p.root().display()))
        })?;
    }

    // No VCS with a checked in `Cargo.toml` found, so we don't know if the
    // directory is dirty or not, thus we have to assume that it's clean.
    return Ok(None);

    fn git(
        p: &Package,
        src_files: &[PathBuf],
        repo: &git2::Repository,
        allow_dirty: bool,
    ) -> CargoResult<Option<String>> {
        let workdir = repo.workdir().unwrap();
        let dirty = src_files
            .iter()
            .filter(|file| {
                let relative = file.strip_prefix(workdir).unwrap();
                if let Ok(status) = repo.status_file(relative) {
                    status != git2::Status::CURRENT
                } else {
                    false
                }
            })
            .map(|path| {
                path.strip_prefix(p.root())
                    .unwrap_or(path)
                    .display()
                    .to_string()
            })
            .collect::<Vec<_>>();
        if dirty.is_empty() {
            let rev_obj = repo.revparse_single("HEAD")?;
            Ok(Some(rev_obj.id().to_string()))
        } else {
            if !allow_dirty {
                failure::bail!(
                    "{} files in the working directory contain changes that were \
                     not yet committed into git:\n\n{}\n\n\
                     to proceed despite this, pass the `--allow-dirty` flag",
                    dirty.len(),
                    dirty.join("\n")
                )
            }
            Ok(None)
        }
    }
}

// Checks for and `bail!` if a source file matches `ROOT/VCS_INFO_FILE`, since
// this is now a Cargo reserved file name, and we don't want to allow forgery.
fn check_vcs_file_collision(pkg: &Package, src_files: &[PathBuf]) -> CargoResult<()> {
    let root = pkg.root();
    let vcs_info_path = Path::new(VCS_INFO_FILE);
    let collision = src_files
        .iter()
        .find(|&p| p.strip_prefix(root).unwrap() == vcs_info_path);
    if collision.is_some() {
        failure::bail!(
            "Invalid inclusion of reserved file name \
             {} in package source",
            VCS_INFO_FILE
        );
    }
    Ok(())
}

fn tar(
    ws: &Workspace<'_>,
    src_files: &[PathBuf],
    vcs_info: Option<&serde_json::Value>,
    dst: &File,
    filename: &str,
) -> CargoResult<()> {
    // Prepare the encoder and its header.
    let filename = Path::new(filename);
    let encoder = GzBuilder::new()
        .filename(util::path2bytes(filename)?)
        .write(dst, Compression::best());

    // Put all package files into a compressed archive.
    let mut ar = Builder::new(encoder);
    let pkg = ws.current()?;
    let config = ws.config();
    let root = pkg.root();

    for src_file in src_files {
        let relative = src_file.strip_prefix(root)?;
        check_filename(relative)?;
        let relative_str = relative.to_str().ok_or_else(|| {
            failure::format_err!("non-utf8 path in source directory: {}", relative.display())
        })?;
        if relative_str == "Cargo.lock" {
            // This is added manually below.
            continue;
        }
        config
            .shell()
            .verbose(|shell| shell.status("Archiving", &relative_str))?;
        let path = format!(
            "{}-{}{}{}",
            pkg.name(),
            pkg.version(),
            path::MAIN_SEPARATOR,
            relative_str
        );

        // The `tar::Builder` type by default will build GNU archives, but
        // unfortunately we force it here to use UStar archives instead. The
        // UStar format has more limitations on the length of path name that it
        // can encode, so it's not quite as nice to use.
        //
        // Older cargos, however, had a bug where GNU archives were interpreted
        // as UStar archives. This bug means that if we publish a GNU archive
        // which has fully filled out metadata it'll be corrupt when unpacked by
        // older cargos.
        //
        // Hopefully in the future after enough cargos have been running around
        // with the bugfixed tar-rs library we'll be able to switch this over to
        // GNU archives, but for now we'll just say that you can't encode paths
        // in archives that are *too* long.
        //
        // For an instance of this in the wild, use the tar-rs 0.3.3 library to
        // unpack the selectors 0.4.0 crate on crates.io. Either that or take a
        // look at rust-lang/cargo#2326.
        let mut header = Header::new_ustar();
        header
            .set_path(&path)
            .chain_err(|| format!("failed to add to archive: `{}`", relative_str))?;
        let mut file = File::open(src_file)
            .chain_err(|| format!("failed to open for archiving: `{}`", src_file.display()))?;
        let metadata = file
            .metadata()
            .chain_err(|| format!("could not learn metadata for: `{}`", relative_str))?;
        header.set_metadata(&metadata);

        if relative_str == "Cargo.toml" {
            let orig = Path::new(&path).with_file_name("Cargo.toml.orig");
            header.set_path(&orig)?;
            header.set_cksum();
            ar.append(&header, &mut file).chain_err(|| {
                internal(format!("could not archive source file `{}`", relative_str))
            })?;

            let mut header = Header::new_ustar();
            let toml = pkg.to_registry_toml(ws.config())?;
            header.set_path(&path)?;
            header.set_entry_type(EntryType::file());
            header.set_mode(0o644);
            header.set_size(toml.len() as u64);
            header.set_cksum();
            ar.append(&header, toml.as_bytes()).chain_err(|| {
                internal(format!("could not archive source file `{}`", relative_str))
            })?;
        } else {
            header.set_cksum();
            ar.append(&header, &mut file).chain_err(|| {
                internal(format!("could not archive source file `{}`", relative_str))
            })?;
        }
    }

    if let Some(json) = vcs_info {
        let filename: PathBuf = Path::new(VCS_INFO_FILE).into();
        debug_assert!(check_filename(&filename).is_ok());
        let fnd = filename.display();
        config
            .shell()
            .verbose(|shell| shell.status("Archiving", &fnd))?;
        let path = format!(
            "{}-{}{}{}",
            pkg.name(),
            pkg.version(),
            path::MAIN_SEPARATOR,
            fnd
        );
        let mut header = Header::new_ustar();
        header
            .set_path(&path)
            .chain_err(|| format!("failed to add to archive: `{}`", fnd))?;
        let json = format!("{}\n", serde_json::to_string_pretty(json)?);
        let mut header = Header::new_ustar();
        header.set_path(&path)?;
        header.set_entry_type(EntryType::file());
        header.set_mode(0o644);
        header.set_size(json.len() as u64);
        header.set_cksum();
        ar.append(&header, json.as_bytes())
            .chain_err(|| internal(format!("could not archive source file `{}`", fnd)))?;
    }

    if pkg.include_lockfile() {
        let new_lock = build_lock(ws)?;

        config
            .shell()
            .verbose(|shell| shell.status("Archiving", "Cargo.lock"))?;
        let path = format!(
            "{}-{}{}Cargo.lock",
            pkg.name(),
            pkg.version(),
            path::MAIN_SEPARATOR
        );
        let mut header = Header::new_ustar();
        header.set_path(&path)?;
        header.set_entry_type(EntryType::file());
        header.set_mode(0o644);
        header.set_size(new_lock.len() as u64);
        header.set_cksum();
        ar.append(&header, new_lock.as_bytes())
            .chain_err(|| internal("could not archive source file `Cargo.lock`"))?;
    }

    let encoder = ar.into_inner()?;
    encoder.finish()?;
    Ok(())
}

/// Generate warnings when packaging Cargo.lock, and the resolve have changed.
fn compare_resolve(
    config: &Config,
    current_pkg: &Package,
    orig_resolve: &Resolve,
    new_resolve: &Resolve,
) -> CargoResult<()> {
    if config.shell().verbosity() != Verbosity::Verbose {
        return Ok(());
    }
    let new_set: BTreeSet<PackageId> = new_resolve.iter().collect();
    let orig_set: BTreeSet<PackageId> = orig_resolve.iter().collect();
    let added = new_set.difference(&orig_set);
    // Removed entries are ignored, this is used to quickly find hints for why
    // an entry changed.
    let removed: Vec<&PackageId> = orig_set.difference(&new_set).collect();
    for pkg_id in added {
        if pkg_id.name() == current_pkg.name() && pkg_id.version() == current_pkg.version() {
            // Skip the package that is being created, since its SourceId
            // (directory) changes.
            continue;
        }
        // Check for candidates where the source has changed (such as [patch]
        // or a dependency with multiple sources like path/version).
        let removed_candidates: Vec<&PackageId> = removed
            .iter()
            .filter(|orig_pkg_id| {
                orig_pkg_id.name() == pkg_id.name() && orig_pkg_id.version() == pkg_id.version()
            })
            .cloned()
            .collect();
        let extra = match removed_candidates.len() {
            0 => {
                // This can happen if the original was out of date.
                let previous_versions: Vec<&PackageId> = removed
                    .iter()
                    .filter(|orig_pkg_id| orig_pkg_id.name() == pkg_id.name())
                    .cloned()
                    .collect();
                match previous_versions.len() {
                    0 => String::new(),
                    1 => format!(
                        ", previous version was `{}`",
                        previous_versions[0].version()
                    ),
                    _ => format!(
                        ", previous versions were: {}",
                        previous_versions
                            .iter()
                            .map(|pkg_id| format!("`{}`", pkg_id.version()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                }
            }
            1 => {
                // This can happen for multi-sourced dependencies like
                // `{path="...", version="..."}` or `[patch]` replacement.
                // `[replace]` is not captured in Cargo.lock.
                format!(
                    ", was originally sourced from `{}`",
                    removed_candidates[0].source_id()
                )
            }
            _ => {
                // I don't know if there is a way to actually trigger this,
                // but handle it just in case.
                let comma_list = removed_candidates
                    .iter()
                    .map(|pkg_id| format!("`{}`", pkg_id.source_id()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    ", was originally sourced from one of these sources: {}",
                    comma_list
                )
            }
        };
        let msg = format!(
            "package `{}` added to the packaged Cargo.lock file{}",
            pkg_id, extra
        );
        config.shell().status_with_color("Note", msg, Color::Cyan)?;
    }
    Ok(())
}

fn check_yanked(config: &Config, pkg_set: &PackageSet<'_>, resolve: &Resolve) -> CargoResult<()> {
    // Checking the yanked status invovles taking a look at the registry and
    // maybe updating files, so be sure to lock it here.
    let _lock = config.acquire_package_cache_lock()?;

    let mut sources = pkg_set.sources_mut();
    for pkg_id in resolve.iter() {
        if let Some(source) = sources.get_mut(pkg_id.source_id()) {
            if source.is_yanked(pkg_id)? {
                config.shell().warn(format!(
                    "package `{}` in Cargo.lock is yanked in registry `{}`, \
                     consider updating to a version that is not yanked",
                    pkg_id,
                    pkg_id.source_id().display_registry_name()
                ))?;
            }
        }
    }
    Ok(())
}

fn run_verify(ws: &Workspace<'_>, tar: &FileLock, opts: &PackageOpts<'_>) -> CargoResult<()> {
    let config = ws.config();
    let pkg = ws.current()?;

    config.shell().status("Verifying", pkg)?;

    let f = GzDecoder::new(tar.file());
    let dst = tar
        .parent()
        .join(&format!("{}-{}", pkg.name(), pkg.version()));
    if dst.exists() {
        paths::remove_dir_all(&dst)?;
    }
    let mut archive = Archive::new(f);
    // We don't need to set the Modified Time, as it's not relevant to verification
    // and it errors on filesystems that don't support setting a modified timestamp
    archive.set_preserve_mtime(false);
    archive.unpack(dst.parent().unwrap())?;

    // Manufacture an ephemeral workspace to ensure that even if the top-level
    // package has a workspace we can still build our new crate.
    let id = SourceId::for_path(&dst)?;
    let mut src = PathSource::new(&dst, id, ws.config());
    let new_pkg = src.root_package()?;
    let pkg_fingerprint = hash_all(&dst)?;
    let ws = Workspace::ephemeral(new_pkg, config, None, true)?;

    let rustc_args = if pkg
        .manifest()
        .features()
        .require(Feature::public_dependency())
        .is_ok()
    {
        // FIXME: Turn this on at some point in the future
        //Some(vec!["-D exported_private_dependencies".to_string()])
        Some(vec![])
    } else {
        None
    };

    let exec: Arc<dyn Executor> = Arc::new(DefaultExecutor);
    ops::compile_ws(
        &ws,
        &ops::CompileOptions {
            config,
            build_config: BuildConfig::new(config, opts.jobs, &opts.target, CompileMode::Build)?,
            features: opts.features.clone(),
            no_default_features: opts.no_default_features,
            all_features: opts.all_features,
            spec: ops::Packages::Packages(Vec::new()),
            filter: ops::CompileFilter::Default {
                required_features_filterable: true,
            },
            target_rustdoc_args: None,
            target_rustc_args: rustc_args,
            local_rustdoc_args: None,
            export_dir: None,
        },
        &exec,
    )?;

    // Check that `build.rs` didn't modify any files in the `src` directory.
    let ws_fingerprint = hash_all(&dst)?;
    if pkg_fingerprint != ws_fingerprint {
        let changes = report_hash_difference(&pkg_fingerprint, &ws_fingerprint);
        failure::bail!(
            "Source directory was modified by build.rs during cargo publish. \
             Build scripts should not modify anything outside of OUT_DIR.\n\
             {}\n\n\
             To proceed despite this, pass the `--no-verify` flag.",
            changes
        )
    }

    Ok(())
}

fn hash_all(path: &Path) -> CargoResult<HashMap<PathBuf, u64>> {
    fn wrap(path: &Path) -> CargoResult<HashMap<PathBuf, u64>> {
        let mut result = HashMap::new();
        let walker = walkdir::WalkDir::new(path).into_iter();
        for entry in walker.filter_entry(|e| !(e.depth() == 1 && e.file_name() == "target")) {
            let entry = entry?;
            let file_type = entry.file_type();
            if file_type.is_file() {
                let contents = fs::read(entry.path())?;
                let hash = util::hex::hash_u64(&contents);
                result.insert(entry.path().to_path_buf(), hash);
            } else if file_type.is_symlink() {
                let hash = util::hex::hash_u64(&fs::read_link(entry.path())?);
                result.insert(entry.path().to_path_buf(), hash);
            } else if file_type.is_dir() {
                let hash = util::hex::hash_u64(&());
                result.insert(entry.path().to_path_buf(), hash);
            }
        }
        Ok(result)
    }
    let result = wrap(path).chain_err(|| format!("failed to verify output at {:?}", path))?;
    Ok(result)
}

fn report_hash_difference(orig: &HashMap<PathBuf, u64>, after: &HashMap<PathBuf, u64>) -> String {
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    for (key, value) in orig {
        match after.get(key) {
            Some(after_value) => {
                if value != after_value {
                    changed.push(key.to_string_lossy());
                }
            }
            None => removed.push(key.to_string_lossy()),
        }
    }
    let mut added: Vec<_> = after
        .keys()
        .filter(|key| !orig.contains_key(*key))
        .map(|key| key.to_string_lossy())
        .collect();
    let mut result = Vec::new();
    if !changed.is_empty() {
        changed.sort_unstable();
        result.push(format!("Changed: {}", changed.join("\n\t")));
    }
    if !added.is_empty() {
        added.sort_unstable();
        result.push(format!("Added: {}", added.join("\n\t")));
    }
    if !removed.is_empty() {
        removed.sort_unstable();
        result.push(format!("Removed: {}", removed.join("\n\t")));
    }
    assert!(!result.is_empty(), "unexpected empty change detection");
    result.join("\n")
}

// It can often be the case that files of a particular name on one platform
// can't actually be created on another platform. For example files with colons
// in the name are allowed on Unix but not on Windows.
//
// To help out in situations like this, issue about weird filenames when
// packaging as a "heads up" that something may not work on other platforms.
fn check_filename(file: &Path) -> CargoResult<()> {
    let name = match file.file_name() {
        Some(name) => name,
        None => return Ok(()),
    };
    let name = match name.to_str() {
        Some(name) => name,
        None => failure::bail!(
            "path does not have a unicode filename which may not unpack \
             on all platforms: {}",
            file.display()
        ),
    };
    let bad_chars = ['/', '\\', '<', '>', ':', '"', '|', '?', '*'];
    if let Some(c) = bad_chars.iter().find(|c| name.contains(**c)) {
        failure::bail!(
            "cannot package a filename with a special character `{}`: {}",
            c,
            file.display()
        )
    }
    Ok(())
}