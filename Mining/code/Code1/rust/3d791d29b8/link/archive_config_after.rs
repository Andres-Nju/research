fn archive_config<'a>(sess: &'a Session,
                      output: &Path,
                      input: Option<&Path>) -> ArchiveConfig<'a> {
    ArchiveConfig {
        sess,
        dst: output.to_path_buf(),
        src: input.map(|p| p.to_path_buf()),
        lib_search_paths: archive_search_paths(sess),
    }
}

fn emit_metadata<'a>(sess: &'a Session, trans: &CrateTranslation, out_filename: &Path) {
    let result = fs::File::create(out_filename).and_then(|mut f| {
        f.write_all(&trans.metadata.raw_data)
    });

    if let Err(e) = result {
        sess.fatal(&format!("failed to write {}: {}", out_filename.display(), e));
    }
}

enum RlibFlavor {
    Normal,
    StaticlibBase,
}

// Create an 'rlib'
//
// An rlib in its current incarnation is essentially a renamed .a file. The
// rlib primarily contains the object file of the crate, but it also contains
// all of the object files from native libraries. This is done by unzipping
// native libraries and inserting all of the contents into this archive.
fn link_rlib<'a>(sess: &'a Session,
                 trans: &CrateTranslation,
                 flavor: RlibFlavor,
                 out_filename: &Path,
                 tmpdir: &Path) -> ArchiveBuilder<'a> {
    info!("preparing rlib to {:?}", out_filename);
    let mut ab = ArchiveBuilder::new(archive_config(sess, out_filename, None));

    for obj in trans.modules.iter().filter_map(|m| m.object.as_ref()) {
        ab.add_file(obj);
    }

    // Note that in this loop we are ignoring the value of `lib.cfg`. That is,
    // we may not be configured to actually include a static library if we're
    // adding it here. That's because later when we consume this rlib we'll
    // decide whether we actually needed the static library or not.
    //
    // To do this "correctly" we'd need to keep track of which libraries added
    // which object files to the archive. We don't do that here, however. The
    // #[link(cfg(..))] feature is unstable, though, and only intended to get
    // liblibc working. In that sense the check below just indicates that if
    // there are any libraries we want to omit object files for at link time we
    // just exclude all custom object files.
    //
    // Eventually if we want to stabilize or flesh out the #[link(cfg(..))]
    // feature then we'll need to figure out how to record what objects were
    // loaded from the libraries found here and then encode that into the
    // metadata of the rlib we're generating somehow.
    for lib in trans.crate_info.used_libraries.iter() {
        match lib.kind {
            NativeLibraryKind::NativeStatic => {}
            NativeLibraryKind::NativeStaticNobundle |
            NativeLibraryKind::NativeFramework |
            NativeLibraryKind::NativeUnknown => continue,
        }
        ab.add_native_library(&lib.name.as_str());
    }

    // After adding all files to the archive, we need to update the
    // symbol table of the archive.
    ab.update_symbols();

    // Note that it is important that we add all of our non-object "magical
    // files" *after* all of the object files in the archive. The reason for
    // this is as follows:
    //
    // * When performing LTO, this archive will be modified to remove
    //   objects from above. The reason for this is described below.
    //
    // * When the system linker looks at an archive, it will attempt to
    //   determine the architecture of the archive in order to see whether its
    //   linkable.
    //
    //   The algorithm for this detection is: iterate over the files in the
    //   archive. Skip magical SYMDEF names. Interpret the first file as an
    //   object file. Read architecture from the object file.
    //
    // * As one can probably see, if "metadata" and "foo.bc" were placed
    //   before all of the objects, then the architecture of this archive would
    //   not be correctly inferred once 'foo.o' is removed.
    //
    // Basically, all this means is that this code should not move above the
    // code above.
    match flavor {
        RlibFlavor::Normal => {
            // Instead of putting the metadata in an object file section, rlibs
            // contain the metadata in a separate file. We use a temp directory
            // here so concurrent builds in the same directory don't try to use
            // the same filename for metadata (stomping over one another)
            let metadata = tmpdir.join(METADATA_FILENAME);
            emit_metadata(sess, trans, &metadata);
            ab.add_file(&metadata);

            // For LTO purposes, the bytecode of this library is also inserted
            // into the archive.
            for bytecode in trans.modules.iter().filter_map(|m| m.bytecode_compressed.as_ref()) {
                ab.add_file(bytecode);
            }

            // After adding all files to the archive, we need to update the
            // symbol table of the archive. This currently dies on macOS (see
            // #11162), and isn't necessary there anyway
            if !sess.target.target.options.is_like_osx {
                ab.update_symbols();
            }
        }

        RlibFlavor::StaticlibBase => {
            let obj = trans.allocator_module
                .as_ref()
                .and_then(|m| m.object.as_ref());
            if let Some(obj) = obj {
                ab.add_file(obj);
            }
        }
    }

    ab
}

// Create a static archive
//
// This is essentially the same thing as an rlib, but it also involves adding
// all of the upstream crates' objects into the archive. This will slurp in
// all of the native libraries of upstream dependencies as well.
//
// Additionally, there's no way for us to link dynamic libraries, so we warn
// about all dynamic library dependencies that they're not linked in.
//
// There's no need to include metadata in a static archive, so ensure to not
// link in the metadata object file (and also don't prepare the archive with a
// metadata file).
fn link_staticlib(sess: &Session,
                  trans: &CrateTranslation,
                  out_filename: &Path,
                  tempdir: &Path) {
    let mut ab = link_rlib(sess,
                           trans,
                           RlibFlavor::StaticlibBase,
                           out_filename,
                           tempdir);
    let mut all_native_libs = vec![];

    let res = each_linked_rlib(sess, &trans.crate_info, &mut |cnum, path| {
        let name = &trans.crate_info.crate_name[&cnum];
        let native_libs = &trans.crate_info.native_libraries[&cnum];

        // Here when we include the rlib into our staticlib we need to make a
        // decision whether to include the extra object files along the way.
        // These extra object files come from statically included native
        // libraries, but they may be cfg'd away with #[link(cfg(..))].
        //
        // This unstable feature, though, only needs liblibc to work. The only
        // use case there is where musl is statically included in liblibc.rlib,
        // so if we don't want the included version we just need to skip it. As
        // a result the logic here is that if *any* linked library is cfg'd away
        // we just skip all object files.
        //
        // Clearly this is not sufficient for a general purpose feature, and
        // we'd want to read from the library's metadata to determine which
        // object files come from where and selectively skip them.
        let skip_object_files = native_libs.iter().any(|lib| {
            lib.kind == NativeLibraryKind::NativeStatic && !relevant_lib(sess, lib)
        });
        ab.add_rlib(path,
                    &name.as_str(),
                    sess.lto() && !ignored_for_lto(&trans.crate_info, cnum),
                    skip_object_files).unwrap();

        all_native_libs.extend(trans.crate_info.native_libraries[&cnum].iter().cloned());
    });
    if let Err(e) = res {
        sess.fatal(&e);
    }

    ab.update_symbols();
    ab.build();

    if !all_native_libs.is_empty() {
        if sess.opts.prints.contains(&PrintRequest::NativeStaticLibs) {
            print_native_static_libs(sess, &all_native_libs);
        }
    }
}

fn print_native_static_libs(sess: &Session, all_native_libs: &[NativeLibrary]) {
    let lib_args: Vec<_> = all_native_libs.iter()
        .filter(|l| relevant_lib(sess, l))
        .filter_map(|lib| match lib.kind {
            NativeLibraryKind::NativeStaticNobundle |
            NativeLibraryKind::NativeUnknown => {
                if sess.target.target.options.is_like_msvc {
                    Some(format!("{}.lib", lib.name))
                } else {
                    Some(format!("-l{}", lib.name))
                }
            },
            NativeLibraryKind::NativeFramework => {
                // ld-only syntax, since there are no frameworks in MSVC
                Some(format!("-framework {}", lib.name))
            },
            // These are included, no need to print them
            NativeLibraryKind::NativeStatic => None,
        })
        .collect();
    if !lib_args.is_empty() {
        sess.note_without_error("Link against the following native artifacts when linking \
                                 against this static library. The order and any duplication \
                                 can be significant on some platforms.");
        // Prefix for greppability
        sess.note_without_error(&format!("native-static-libs: {}", &lib_args.join(" ")));
    }
}

// Create a dynamic library or executable
//
// This will invoke the system linker/cc to create the resulting file. This
// links to all upstream files as well.
fn link_natively(sess: &Session,
                 crate_type: config::CrateType,
                 out_filename: &Path,
                 trans: &CrateTranslation,
                 tmpdir: &Path) {
    info!("preparing {:?} to {:?}", crate_type, out_filename);
    let flavor = sess.linker_flavor();

    // The invocations of cc share some flags across platforms
    let (pname, mut cmd, envs) = get_linker(sess);
    // This will set PATH on windows
    cmd.envs(envs);

    let root = sess.target_filesearch(PathKind::Native).get_lib_path();
    if let Some(args) = sess.target.target.options.pre_link_args.get(&flavor) {
        cmd.args(args);
    }
    if let Some(ref args) = sess.opts.debugging_opts.pre_link_args {
        cmd.args(args);
    }
    cmd.args(&sess.opts.debugging_opts.pre_link_arg);

    let pre_link_objects = if crate_type == config::CrateTypeExecutable {
        &sess.target.target.options.pre_link_objects_exe
    } else {
        &sess.target.target.options.pre_link_objects_dll
    };
    for obj in pre_link_objects {
        cmd.arg(root.join(obj));
    }

    if sess.target.target.options.is_like_emscripten {
        cmd.arg("-s");
        cmd.arg(if sess.panic_strategy() == PanicStrategy::Abort {
            "DISABLE_EXCEPTION_CATCHING=1"
        } else {
            "DISABLE_EXCEPTION_CATCHING=0"
        });
    }

    {
        let mut linker = trans.linker_info.to_linker(cmd, &sess);
        link_args(&mut *linker, sess, crate_type, tmpdir,
                  out_filename, trans);
        cmd = linker.finalize();
    }
    if let Some(args) = sess.target.target.options.late_link_args.get(&flavor) {
        cmd.args(args);
    }
    for obj in &sess.target.target.options.post_link_objects {
        cmd.arg(root.join(obj));
    }
    if let Some(args) = sess.target.target.options.post_link_args.get(&flavor) {
        cmd.args(args);
    }
    for &(ref k, ref v) in &sess.target.target.options.link_env {
        cmd.env(k, v);
    }

    if sess.opts.debugging_opts.print_link_args {
        println!("{:?}", &cmd);
    }

    // May have not found libraries in the right formats.
    sess.abort_if_errors();

    // Invoke the system linker
    //
    // Note that there's a terribly awful hack that really shouldn't be present
    // in any compiler. Here an environment variable is supported to
    // automatically retry the linker invocation if the linker looks like it
    // segfaulted.
    //
    // Gee that seems odd, normally segfaults are things we want to know about!
    // Unfortunately though in rust-lang/rust#38878 we're experiencing the
    // linker segfaulting on Travis quite a bit which is causing quite a bit of
    // pain to land PRs when they spuriously fail due to a segfault.
    //
    // The issue #38878 has some more debugging information on it as well, but
    // this unfortunately looks like it's just a race condition in macOS's linker
    // with some thread pool working in the background. It seems that no one
    // currently knows a fix for this so in the meantime we're left with this...
    info!("{:?}", &cmd);
    let retry_on_segfault = env::var("RUSTC_RETRY_LINKER_ON_SEGFAULT").is_ok();
    let mut prog;
    let mut i = 0;
    loop {
        i += 1;
        prog = time(sess.time_passes(), "running linker", || {
            exec_linker(sess, &mut cmd, tmpdir)
        });
        if !retry_on_segfault || i > 3 {
            break
        }
        let output = match prog {
            Ok(ref output) => output,
            Err(_) => break,
        };
        if output.status.success() {
            break
        }
        let mut out = output.stderr.clone();
        out.extend(&output.stdout);
        let out = String::from_utf8_lossy(&out);
        let msg_segv = "clang: error: unable to execute command: Segmentation fault: 11";
        let msg_bus  = "clang: error: unable to execute command: Bus error: 10";
        if !(out.contains(msg_segv) || out.contains(msg_bus)) {
            break
        }

        warn!(
            "looks like the linker segfaulted when we tried to call it, \
             automatically retrying again. cmd = {:?}, out = {}.",
            cmd,
            out,
        );
    }

    match prog {
        Ok(prog) => {
            fn escape_string(s: &[u8]) -> String {
                str::from_utf8(s).map(|s| s.to_owned())
                    .unwrap_or_else(|_| {
                        let mut x = "Non-UTF-8 output: ".to_string();
                        x.extend(s.iter()
                                 .flat_map(|&b| ascii::escape_default(b))
                                 .map(|b| char::from_u32(b as u32).unwrap()));
                        x
                    })
            }
            if !prog.status.success() {
                let mut output = prog.stderr.clone();
                output.extend_from_slice(&prog.stdout);
                sess.struct_err(&format!("linking with `{}` failed: {}",
                                         pname,
                                         prog.status))
                    .note(&format!("{:?}", &cmd))
                    .note(&escape_string(&output))
                    .emit();
                sess.abort_if_errors();
            }
            info!("linker stderr:\n{}", escape_string(&prog.stderr));
            info!("linker stdout:\n{}", escape_string(&prog.stdout));
        },
        Err(e) => {
            sess.struct_err(&format!("could not exec the linker `{}`: {}", pname, e))
                .note(&format!("{:?}", &cmd))
                .emit();
            if sess.target.target.options.is_like_msvc && e.kind() == io::ErrorKind::NotFound {
                sess.note_without_error("the msvc targets depend on the msvc linker \
                    but `link.exe` was not found");
                sess.note_without_error("please ensure that VS 2013 or VS 2015 was installed \
                    with the Visual C++ option");
            }
            sess.abort_if_errors();
        }
    }


    // On macOS, debuggers need this utility to get run to do some munging of
    // the symbols
    if sess.target.target.options.is_like_osx && sess.opts.debuginfo != NoDebugInfo {
        match Command::new("dsymutil").arg(out_filename).output() {
            Ok(..) => {}
            Err(e) => sess.fatal(&format!("failed to run dsymutil: {}", e)),
        }
    }
}

fn exec_linker(sess: &Session, cmd: &mut Command, tmpdir: &Path)
    -> io::Result<Output>
{
    // When attempting to spawn the linker we run a risk of blowing out the
    // size limits for spawning a new process with respect to the arguments
    // we pass on the command line.
    //
    // Here we attempt to handle errors from the OS saying "your list of
    // arguments is too big" by reinvoking the linker again with an `@`-file
    // that contains all the arguments. The theory is that this is then
    // accepted on all linkers and the linker will read all its options out of
    // there instead of looking at the command line.
    match cmd.command().stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(child) => return child.wait_with_output(),
        Err(ref e) if command_line_too_big(e) => {}
        Err(e) => return Err(e)
    }

    let file = tmpdir.join("linker-arguments");
    let mut cmd2 = Command::new(cmd.get_program());
    cmd2.arg(format!("@{}", file.display()));
    for &(ref k, ref v) in cmd.get_env() {
        cmd2.env(k, v);
    }
    let mut f = BufWriter::new(File::create(&file)?);
    for arg in cmd.get_args() {
        writeln!(f, "{}", Escape {
            arg: arg.to_str().unwrap(),
            is_like_msvc: sess.target.target.options.is_like_msvc,
        })?;
    }
    f.into_inner()?;
    return cmd2.output();

    #[cfg(unix)]
    fn command_line_too_big(err: &io::Error) -> bool {
        err.raw_os_error() == Some(::libc::E2BIG)
    }

    #[cfg(windows)]
    fn command_line_too_big(err: &io::Error) -> bool {
        const ERROR_FILENAME_EXCED_RANGE: i32 = 206;
        err.raw_os_error() == Some(ERROR_FILENAME_EXCED_RANGE)
    }

    struct Escape<'a> {
        arg: &'a str,
        is_like_msvc: bool,
    }

    impl<'a> fmt::Display for Escape<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            if self.is_like_msvc {
                // This is "documented" at
                // https://msdn.microsoft.com/en-us/library/4xdcbak7.aspx
                //
                // Unfortunately there's not a great specification of the
                // syntax I could find online (at least) but some local
                // testing showed that this seemed sufficient-ish to catch
                // at least a few edge cases.
                write!(f, "\"")?;
                for c in self.arg.chars() {
                    match c {
                        '"' => write!(f, "\\{}", c)?,
                        c => write!(f, "{}", c)?,
                    }
                }
                write!(f, "\"")?;
            } else {
                // This is documented at https://linux.die.net/man/1/ld, namely:
                //
                // > Options in file are separated by whitespace. A whitespace
                // > character may be included in an option by surrounding the
                // > entire option in either single or double quotes. Any
                // > character (including a backslash) may be included by
                // > prefixing the character to be included with a backslash.
                //
                // We put an argument on each line, so all we need to do is
                // ensure the line is interpreted as one whole argument.
                for c in self.arg.chars() {
                    match c {
                        '\\' |
                        ' ' => write!(f, "\\{}", c)?,
                        c => write!(f, "{}", c)?,
                    }
                }
            }
            Ok(())
        }
    }
}

fn link_args(cmd: &mut Linker,
             sess: &Session,
             crate_type: config::CrateType,
             tmpdir: &Path,
             out_filename: &Path,
             trans: &CrateTranslation) {

    // The default library location, we need this to find the runtime.
    // The location of crates will be determined as needed.
    let lib_path = sess.target_filesearch(PathKind::All).get_lib_path();

    // target descriptor
    let t = &sess.target.target;

    cmd.include_path(&fix_windows_verbatim_for_gcc(&lib_path));
    for obj in trans.modules.iter().filter_map(|m| m.object.as_ref()) {
        cmd.add_object(obj);
    }
    cmd.output_filename(out_filename);

    if crate_type == config::CrateTypeExecutable &&
       sess.target.target.options.is_like_windows {
        if let Some(ref s) = trans.windows_subsystem {
            cmd.subsystem(s);
        }
    }

    // If we're building a dynamic library then some platforms need to make sure
    // that all symbols are exported correctly from the dynamic library.
    if crate_type != config::CrateTypeExecutable ||
       sess.target.target.options.is_like_emscripten {
        cmd.export_symbols(tmpdir, crate_type);
    }

    // When linking a dynamic library, we put the metadata into a section of the
    // executable. This metadata is in a separate object file from the main
    // object file, so we link that in here.
    if crate_type == config::CrateTypeDylib ||
       crate_type == config::CrateTypeProcMacro {
        if let Some(obj) = trans.metadata_module.object.as_ref() {
            cmd.add_object(obj);
        }
    }

    let obj = trans.allocator_module
        .as_ref()
        .and_then(|m| m.object.as_ref());
    if let Some(obj) = obj {
        cmd.add_object(obj);
    }

    // Try to strip as much out of the generated object by removing unused
    // sections if possible. See more comments in linker.rs
    if !sess.opts.cg.link_dead_code {
        let keep_metadata = crate_type == config::CrateTypeDylib;
        cmd.gc_sections(keep_metadata);
    }

    let used_link_args = &trans.crate_info.link_args;

    if crate_type == config::CrateTypeExecutable &&
       t.options.position_independent_executables {
        let empty_vec = Vec::new();
        let args = sess.opts.cg.link_args.as_ref().unwrap_or(&empty_vec);
        let more_args = &sess.opts.cg.link_arg;
        let mut args = args.iter().chain(more_args.iter()).chain(used_link_args.iter());

        if get_reloc_model(sess) == llvm::RelocMode::PIC
            && !sess.crt_static() && !args.any(|x| *x == "-static") {
            cmd.position_independent_executable();
        }
    }

    let relro_level = match sess.opts.debugging_opts.relro_level {
        Some(level) => level,
        None => t.options.relro_level,
    };
    match relro_level {
        RelroLevel::Full => {
            cmd.full_relro();
        },
        RelroLevel::Partial => {
            cmd.partial_relro();
        },
        RelroLevel::Off => {},
    }

    // Pass optimization flags down to the linker.
    cmd.optimize();

    // Pass debuginfo flags down to the linker.
    cmd.debuginfo();

    // We want to prevent the compiler from accidentally leaking in any system
    // libraries, so we explicitly ask gcc to not link to any libraries by
    // default. Note that this does not happen for windows because windows pulls
    // in some large number of libraries and I couldn't quite figure out which
    // subset we wanted.
    if t.options.no_default_libraries {
        cmd.no_default_libraries();
    }

    // Take careful note of the ordering of the arguments we pass to the linker
    // here. Linkers will assume that things on the left depend on things to the
    // right. Things on the right cannot depend on things on the left. This is
    // all formally implemented in terms of resolving symbols (libs on the right
    // resolve unknown symbols of libs on the left, but not vice versa).
    //
    // For this reason, we have organized the arguments we pass to the linker as
    // such:
    //
    //  1. The local object that LLVM just generated
    //  2. Local native libraries
    //  3. Upstream rust libraries
    //  4. Upstream native libraries
    //
    // The rationale behind this ordering is that those items lower down in the
    // list can't depend on items higher up in the list. For example nothing can
    // depend on what we just generated (e.g. that'd be a circular dependency).
    // Upstream rust libraries are not allowed to depend on our local native
    // libraries as that would violate the structure of the DAG, in that
    // scenario they are required to link to them as well in a shared fashion.
    //
    // Note that upstream rust libraries may contain native dependencies as
    // well, but they also can't depend on what we just started to add to the
    // link line. And finally upstream native libraries can't depend on anything
    // in this DAG so far because they're only dylibs and dylibs can only depend
    // on other dylibs (e.g. other native deps).
    add_local_native_libraries(cmd, sess, trans);
    add_upstream_rust_crates(cmd, sess, trans, crate_type, tmpdir);
    add_upstream_native_libraries(cmd, sess, trans, crate_type);

    // Tell the linker what we're doing.
    if crate_type != config::CrateTypeExecutable {
        cmd.build_dylib(out_filename);
    }
    if crate_type == config::CrateTypeExecutable && sess.crt_static() {
        cmd.build_static_executable();
    }

    // FIXME (#2397): At some point we want to rpath our guesses as to
    // where extern libraries might live, based on the
    // addl_lib_search_paths
    if sess.opts.cg.rpath {
        let sysroot = sess.sysroot();
        let target_triple = &sess.opts.target_triple;
        let mut get_install_prefix_lib_path = || {
            let install_prefix = option_env!("CFG_PREFIX").expect("CFG_PREFIX");
            let tlib = filesearch::relative_target_lib_path(sysroot, target_triple);
            let mut path = PathBuf::from(install_prefix);
            path.push(&tlib);

            path
        };
        let mut rpath_config = RPathConfig {
            used_crates: &trans.crate_info.used_crates_dynamic,
            out_filename: out_filename.to_path_buf(),
            has_rpath: sess.target.target.options.has_rpath,
            is_like_osx: sess.target.target.options.is_like_osx,
            linker_is_gnu: sess.target.target.options.linker_is_gnu,
            get_install_prefix_lib_path: &mut get_install_prefix_lib_path,
        };
        cmd.args(&rpath::get_rpath_flags(&mut rpath_config));
    }

    // Finally add all the linker arguments provided on the command line along
    // with any #[link_args] attributes found inside the crate
    if let Some(ref args) = sess.opts.cg.link_args {
        cmd.args(args);
    }
    cmd.args(&sess.opts.cg.link_arg);
    cmd.args(&used_link_args);
}

// # Native library linking
//
// User-supplied library search paths (-L on the command line). These are
// the same paths used to find Rust crates, so some of them may have been
// added already by the previous crate linking code. This only allows them
// to be found at compile time so it is still entirely up to outside
// forces to make sure that library can be found at runtime.
//
// Also note that the native libraries linked here are only the ones located
// in the current crate. Upstream crates with native library dependencies
// may have their native library pulled in above.
fn add_local_native_libraries(cmd: &mut Linker,
                              sess: &Session,
                              trans: &CrateTranslation) {
    sess.target_filesearch(PathKind::All).for_each_lib_search_path(|path, k| {
        match k {
            PathKind::Framework => { cmd.framework_path(path); }
            _ => { cmd.include_path(&fix_windows_verbatim_for_gcc(path)); }
        }
    });

    let relevant_libs = trans.crate_info.used_libraries.iter().filter(|l| {
        relevant_lib(sess, l)
    });

    let search_path = archive_search_paths(sess);
    for lib in relevant_libs {
        match lib.kind {
            NativeLibraryKind::NativeUnknown => cmd.link_dylib(&lib.name.as_str()),
            NativeLibraryKind::NativeFramework => cmd.link_framework(&lib.name.as_str()),
            NativeLibraryKind::NativeStaticNobundle => cmd.link_staticlib(&lib.name.as_str()),
            NativeLibraryKind::NativeStatic => cmd.link_whole_staticlib(&lib.name.as_str(),
                                                                        &search_path)
        }
    }
}

// # Rust Crate linking
//
// Rust crates are not considered at all when creating an rlib output. All
// dependencies will be linked when producing the final output (instead of
// the intermediate rlib version)
fn add_upstream_rust_crates(cmd: &mut Linker,
                            sess: &Session,
                            trans: &CrateTranslation,
                            crate_type: config::CrateType,
                            tmpdir: &Path) {
    // All of the heavy lifting has previously been accomplished by the
    // dependency_format module of the compiler. This is just crawling the
    // output of that module, adding crates as necessary.
    //
    // Linking to a rlib involves just passing it to the linker (the linker
    // will slurp up the object files inside), and linking to a dynamic library
    // involves just passing the right -l flag.

    let formats = sess.dependency_formats.borrow();
    let data = formats.get(&crate_type).unwrap();

    // Invoke get_used_crates to ensure that we get a topological sorting of
    // crates.
    let deps = &trans.crate_info.used_crates_dynamic;

    let mut compiler_builtins = None;

    for &(cnum, _) in deps.iter() {
        // We may not pass all crates through to the linker. Some crates may
        // appear statically in an existing dylib, meaning we'll pick up all the
        // symbols from the dylib.
        let src = &trans.crate_info.used_crate_source[&cnum];
        match data[cnum.as_usize() - 1] {
            _ if trans.crate_info.profiler_runtime == Some(cnum) => {
                add_static_crate(cmd, sess, trans, tmpdir, crate_type, cnum);
            }
            _ if trans.crate_info.sanitizer_runtime == Some(cnum) => {
                link_sanitizer_runtime(cmd, sess, trans, tmpdir, cnum);
            }
            // compiler-builtins are always placed last to ensure that they're
            // linked correctly.
            _ if trans.crate_info.compiler_builtins == Some(cnum) => {
                assert!(compiler_builtins.is_none());
                compiler_builtins = Some(cnum);
            }
            Linkage::NotLinked |
            Linkage::IncludedFromDylib => {}
            Linkage::Static => {
                add_static_crate(cmd, sess, trans, tmpdir, crate_type, cnum);
            }
            Linkage::Dynamic => {
                add_dynamic_crate(cmd, sess, &src.dylib.as_ref().unwrap().0)
            }
        }
    }

    // compiler-builtins are always placed last to ensure that they're
    // linked correctly.
    // We must always link the `compiler_builtins` crate statically. Even if it
    // was already "included" in a dylib (e.g. `libstd` when `-C prefer-dynamic`
    // is used)
    if let Some(cnum) = compiler_builtins {
        add_static_crate(cmd, sess, trans, tmpdir, crate_type, cnum);
    }

    // Converts a library file-stem into a cc -l argument
    fn unlib<'a>(config: &config::Config, stem: &'a str) -> &'a str {
        if stem.starts_with("lib") && !config.target.options.is_like_windows {
            &stem[3..]
        } else {
            stem
        }
    }

    // We must link the sanitizer runtime using -Wl,--whole-archive but since
    // it's packed in a .rlib, it contains stuff that are not objects that will
    // make the linker error. So we must remove those bits from the .rlib before
    // linking it.
    fn link_sanitizer_runtime(cmd: &mut Linker,
                              sess: &Session,
                              trans: &CrateTranslation,
                              tmpdir: &Path,
                              cnum: CrateNum) {
        let src = &trans.crate_info.used_crate_source[&cnum];
        let cratepath = &src.rlib.as_ref().unwrap().0;

        if sess.target.target.options.is_like_osx {
            // On Apple platforms, the sanitizer is always built as a dylib, and
            // LLVM will link to `@rpath/*.dylib`, so we need to specify an
            // rpath to the library as well (the rpath should be absolute, see
            // PR #41352 for details).
            //
            // FIXME: Remove this logic into librustc_*san once Cargo supports it
            let rpath = cratepath.parent().unwrap();
            let rpath = rpath.to_str().expect("non-utf8 component in path");
            cmd.args(&["-Wl,-rpath".into(), "-Xlinker".into(), rpath.into()]);
        }

        let dst = tmpdir.join(cratepath.file_name().unwrap());
        let cfg = archive_config(sess, &dst, Some(cratepath));
        let mut archive = ArchiveBuilder::new(cfg);
        archive.update_symbols();

        for f in archive.src_files() {
            if f.ends_with(RLIB_BYTECODE_EXTENSION) || f == METADATA_FILENAME {
                archive.remove_file(&f);
                continue
            }
        }

        archive.build();

        cmd.link_whole_rlib(&dst);
    }