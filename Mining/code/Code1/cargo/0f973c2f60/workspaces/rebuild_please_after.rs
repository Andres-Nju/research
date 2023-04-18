fn rebuild_please() {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [workspace]
            members = ['lib', 'bin']
        "#)
        .file("lib/Cargo.toml", r#"
            [package]
            name = "lib"
            version = "0.1.0"
        "#)
        .file("lib/src/lib.rs", r#"
            pub fn foo() -> u32 { 0 }
        "#)
        .file("bin/Cargo.toml", r#"
            [package]
            name = "bin"
            version = "0.1.0"

            [dependencies]
            lib = { path = "../lib" }
        "#)
        .file("bin/src/main.rs", r#"
            extern crate lib;

            fn main() {
                assert_eq!(lib::foo(), 0);
            }
        "#);
    p.build();

    assert_that(p.cargo("run").cwd(p.root().join("bin")),
                execs().with_status(0));

    sleep_ms(1000);

    t!(t!(File::create(p.root().join("lib/src/lib.rs"))).write_all(br#"
        pub fn foo() -> u32 { 1 }
    "#));

    assert_that(p.cargo("build").cwd(p.root().join("lib")),
                execs().with_status(0));

    assert_that(p.cargo("run").cwd(p.root().join("bin")),
                execs().with_status(101));
}
