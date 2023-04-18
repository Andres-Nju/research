pub fn handle_code_action(
    world: ServerWorld,
    params: req::CodeActionParams,
) -> Result<Option<CodeActionResponse>> {
    let file_id = params.text_document.try_conv_with(&world)?;
    let line_index = world.analysis().file_line_index(file_id);
    let range = params.range.conv_with(&line_index);

    let assists = world.analysis().assists(file_id, range)?.into_iter();
    let fixes = world
        .analysis()
        .diagnostics(file_id)?
        .into_iter()
        .filter_map(|d| Some((d.range, d.fix?)))
        .filter(|(range, _fix)| contains_offset_nonstrict(*range, range.start()))
        .map(|(_range, fix)| fix);

    let mut res = Vec::new();
    for source_edit in assists.chain(fixes) {
        let title = source_edit.label.clone();
        let edit = source_edit.try_conv_with(&world)?;
        let cmd = Command {
            title,
            command: "ra-lsp.applySourceChange".to_string(),
            arguments: Some(vec![to_value(edit).unwrap()]),
        };
        res.push(cmd);
    }

    Ok(Some(CodeActionResponse::Commands(res)))
}
