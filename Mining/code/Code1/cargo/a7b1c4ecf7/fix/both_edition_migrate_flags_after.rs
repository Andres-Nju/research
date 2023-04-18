    let stderr = "\
error: The argument '--edition' cannot be used with '--prepare-for <prepare-for>'

USAGE:
    cargo[..] fix --edition --message-format <FMT>

For more information try --help
";

    p.cargo("fix --prepare-for 2018 --edition")
        .with_status(1)
        .with_stderr(stderr)
        .run();
}

#[test]
fn shows_warnings_on_second_run_without_changes() {
    let p = project()
        .file(
            "src/lib.rs",
