fn no_index_update() {
    use cargotest::ChannelChanger;
    let p = project("foo")
        .file("Cargo.toml", r#"
            [package]
            name = "foo"
            authors = []
            version = "0.0.1"

            [dependencies]
            serde = "1.0"
        "#)
        .file("src/main.rs", "fn main() {}")
        .build();

    assert_that(p.cargo("generate-lockfile"),
                execs().with_status(0).with_stdout("")
                    .with_stderr_contains("    Updating registry `https://github.com/rust-lang/crates.io-index`"));

    assert_that(p.cargo("generate-lockfile").masquerade_as_nightly_cargo().arg("-Zno-index-update"),
                execs().with_status(0).with_stdout("")
                    .with_stderr_does_not_contain("    Updating registry `https://github.com/rust-lang/crates.io-index`"));
}
