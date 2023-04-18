pub fn version(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.args.span;

    let mut indexmap = IndexMap::with_capacity(4);

    indexmap.insert(
        "version".to_string(),
        UntaggedValue::string(clap::crate_version!()).into_value(&tag),
    );

    let commit_hash = Some(GIT_COMMIT_HASH.trim()).filter(|x| !x.is_empty());
    if let Some(commit_hash) = commit_hash {
        indexmap.insert(
            "commit_hash".to_string(),
            UntaggedValue::string(commit_hash).into_value(&tag),
        );
    }

    indexmap.insert("features".to_string(), features_enabled(&tag).into_value());

    let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);
    Ok(OutputStream::one(value))
}
