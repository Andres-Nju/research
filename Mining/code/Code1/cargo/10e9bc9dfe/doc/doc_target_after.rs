fn doc_target() {
    if !is_nightly() {
        // no_core, lang_items requires nightly.
        return;
    }
    const TARGET: &str = "arm-unknown-linux-gnueabihf";

    let p = project()
        .file(
            "src/lib.rs",
            r#"
            #![feature(no_core, lang_items)]
            #![no_core]

            #[lang = "sized"]
            trait Sized {}

            extern {
                pub static A: u32;
            }
        "#,
        )
        .build();

    p.cargo("doc --verbose --target").arg(TARGET).run();
    assert!(p.root().join(&format!("target/{}/doc", TARGET)).is_dir());
    assert!(p
        .root()
        .join(&format!("target/{}/doc/foo/index.html", TARGET))
        .is_file());
}
