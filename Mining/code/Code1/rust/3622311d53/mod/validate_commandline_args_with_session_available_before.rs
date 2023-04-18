fn validate_commandline_args_with_session_available(sess: &Session) {
    // Since we don't know if code in an rlib will be linked to statically or
    // dynamically downstream, rustc generates `__imp_` symbols that help the
    // MSVC linker deal with this lack of knowledge (#27438). Unfortunately,
    // these manually generated symbols confuse LLD when it tries to merge
    // bitcode during ThinLTO. Therefore we disallow dynamic linking on MSVC
    // when compiling for LLD ThinLTO. This way we can validly just not generate
    // the `dllimport` attributes and `__imp_` symbols in that case.
    if sess.opts.cg.linker_plugin_lto.enabled() &&
       sess.opts.cg.prefer_dynamic &&
       sess.target.target.options.is_like_msvc {
        sess.err("Linker plugin based LTO is not supported together with \
                  `-C prefer-dynamic` when targeting MSVC");
    }

    // Make sure that any given profiling data actually exists so LLVM can't
    // decide to silently skip PGO.
    if let Some(ref path) = sess.opts.cg.profile_use {
        if !path.exists() {
            sess.err(&format!("File `{}` passed to `-C profile-use` does not exist.",
                              path.display()));
        }
    }

    // PGO does not work reliably with panic=unwind on Windows. Let's make it
    // an error to combine the two for now. It always runs into an assertions
    // if LLVM is built with assertions, but without assertions it sometimes
    // does not crash and will probably generate a corrupted binary.
    if sess.opts.cg.profile_generate.enabled() &&
       sess.target.target.options.is_like_msvc &&
       sess.panic_strategy() == PanicStrategy::Unwind {
        sess.err("Profile-guided optimization does not yet work in conjunction \
                  with `-Cpanic=unwind` on Windows when targeting MSVC. \
                  See https://github.com/rust-lang/rust/issues/61002 for details.");
    }
}
