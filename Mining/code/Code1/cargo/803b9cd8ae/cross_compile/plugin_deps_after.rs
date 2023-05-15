fn plugin_deps() {
    if cross_compile::disabled() {
        return;
    }
    if !is_nightly() {
        // plugins are unstable
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
    let _bar = project()
        .at("bar")
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
        "#,
        )
        .file(
            "src/lib.rs",
            r#"
            #![feature(plugin_registrar, rustc_private)]

            extern crate rustc_driver;
            extern crate syntax;

            use rustc_driver::plugin::Registry;
            use syntax::tokenstream::TokenStream;
            use syntax::source_map::Span;
            use syntax::ast::*;
            use syntax::ext::base::{ExtCtxt, MacEager, MacResult};

            #[plugin_registrar]
            pub fn foo(reg: &mut Registry) {
                reg.register_macro("bar", expand_bar);
            }

            fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: TokenStream)
                          -> Box<MacResult + 'static> {
                MacEager::expr(cx.expr_lit(sp, LitKind::Int(1, LitIntType::Unsuffixed)))
            }
        "#,
        )
        .build();
    let _baz = project()
        .at("baz")
        .file("Cargo.toml", &basic_manifest("baz", "0.0.1"))
        .file("src/lib.rs", "pub fn baz() -> i32 { 1 }")
        .build();

    let target = cross_compile::alternate();
    foo.cargo("build --target").arg(&target).run();
    assert!(foo.target_bin(&target, "foo").is_file());

    foo.process(&foo.target_bin(&target, "foo")).run();
}