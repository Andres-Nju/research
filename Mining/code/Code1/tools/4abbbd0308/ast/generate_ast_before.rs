pub fn generate_ast(mode: Mode, language_kind_list: Vec<String>) -> Result<()> {
    let codegen_language_kinds = if language_kind_list.is_empty() {
        ALL_LANGUAGE_KIND.clone().to_vec()
    } else {
        language_kind_list
            .iter()
            .filter_map(|kind| match LanguageKind::from_str(kind) {
                Ok(kind) => Some(kind),
                Err(err) => {
                    println_string_with_fg_color(err, Color::Red);
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    for kind in codegen_language_kinds {
        println_string_with_fg_color(
            format!(
                "-------------------Generating AST for {}-------------------",
                kind
            ),
            Color::Green,
        );
        let mut ast = match kind {
            LanguageKind::Js => load_js_ast(),
            LanguageKind::Css => load_css_ast(),
            LanguageKind::Json => load_json_ast(),
        };
        ast.sort();
        generate_syntax(ast, &mode, kind)?;
    }

    Ok(())
}
