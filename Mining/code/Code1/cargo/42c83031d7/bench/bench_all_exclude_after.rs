fn bench_all_exclude() {
    if !is_nightly() { return }

    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]
            name = "foo"
            version = "0.1.0"

            [workspace]
            members = ["bar", "baz"]
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#)
        .file("bar/Cargo.toml", r#"
            [project]
            name = "bar"
            version = "0.1.0"
        "#)
        .file("bar/src/lib.rs", r#"
            #![feature(test)]

            extern crate test;

            #[bench]
            pub fn bar(b: &mut test::Bencher) {
                b.iter(|| {});
            }
        "#)
        .file("baz/Cargo.toml", r#"
            [project]
            name = "baz"
            version = "0.1.0"
        "#)
        .file("baz/src/lib.rs", r#"
            #[test]
            pub fn baz() {
                break_the_build();
            }
        "#);

    assert_that(p.cargo_process("bench")
                    .arg("--all")
                    .arg("--exclude")
                    .arg("baz"),
                execs().with_status(0)
                    .with_stdout_contains("\
running 1 test
test bar ... bench:           [..] ns/iter (+/- [..])"));
}
