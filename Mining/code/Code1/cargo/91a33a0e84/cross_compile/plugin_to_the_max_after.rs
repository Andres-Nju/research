fn plugin_to_the_max() {
    if cross_compile::disabled() {
        return;
    }
    if !is_nightly() {
        return;
    }

    let foo = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies.bar]
            path = "../bar"

            [dependencies.baz]
            path = "../baz"
        "#,
        )
        .file(
            "src/main.rs",
            r#"
            #![feature(plugin)]
            #![plugin(bar)]
            extern crate baz;
            fn main() {
                assert_eq!(bar!(), baz::baz());
            }
        "#,
        )
        .build();
    let _bar = project().at("bar")
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "bar"
            version = "0.0.1"
            authors = []

            [lib]
            name = "bar"
            plugin = true

            [dependencies.baz]
            path = "../baz"
        "#,
        )
        .file(
            "src/lib.rs",
            r#"
            #![feature(plugin_registrar, rustc_private)]

            extern crate rustc_plugin;
            extern crate syntax;
            extern crate baz;

            use rustc_plugin::Registry;
            use syntax::tokenstream::TokenTree;
            use syntax::source_map::Span;
            use syntax::ast::*;
            use syntax::ext::base::{ExtCtxt, MacEager, MacResult};
            use syntax::ext::build::AstBuilder;
            use syntax::ptr::P;

            #[plugin_registrar]
            pub fn foo(reg: &mut Registry) {
                reg.register_macro("bar", expand_bar);
            }

            fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree])
                          -> Box<MacResult + 'static> {
                let bar = Ident::from_str("baz");
                let path = cx.path(sp, vec![bar.clone(), bar]);
                MacEager::expr(cx.expr_call(sp, cx.expr_path(path), vec![]))
            }
        "#,
        )
        .build();
    let _baz = project().at("baz")
        .file("Cargo.toml",  &basic_manifest("baz", "0.0.1"))
        .file("src/lib.rs", "pub fn baz() -> i32 { 1 }")
        .build();

    let target = cross_compile::alternate();
    assert_that(
        foo.cargo("build -v --target").arg(&target),
        execs(),
    );
    println!("second");
    assert_that(
        foo.cargo("build -v --target").arg(&target),
        execs(),
    );
    assert_that(&foo.target_bin(&target, "foo"), existing_file());

    assert_that(
        process(&foo.target_bin(&target, "foo")),
        execs(),
    );
}
