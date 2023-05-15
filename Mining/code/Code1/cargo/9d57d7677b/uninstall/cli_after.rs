pub fn cli() -> App {
    subcommand("uninstall")
        .about("Remove a Rust binary")
        .arg(Arg::with_name("spec").multiple(true))
        .arg_package_spec_simple("Package to uninstall")
        .arg(multi_opt("bin", "NAME", "Only uninstall the binary NAME"))
        .arg(opt("root", "Directory to uninstall packages from").value_name("DIR"))
        .after_help(
            "\
The argument SPEC is a package id specification (see `cargo help pkgid`) to
specify which crate should be uninstalled. By default all binaries are
uninstalled for a crate but the `--bin` and `--example` flags can be used to
only uninstall particular binaries.
",
        )
}