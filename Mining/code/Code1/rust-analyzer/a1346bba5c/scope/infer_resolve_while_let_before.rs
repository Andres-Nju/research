    fn infer_resolve_while_let() {
        covers!(infer_resolve_while_let);
        do_check_local_name(
            r#"
fn test() {
    let foo: Option<f32> = None;
    while let Option::Some(spam) = foo {
        spam<|>
    }
}
"#,
            75,
        );
    }
