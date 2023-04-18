fn shells(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let mut shells_out = VecDeque::new();
    let tag = args.call_info.name_tag;

    for (index, shell) in args.shell_manager.shells.lock().iter().enumerate() {
        let mut dict = TaggedDictBuilder::new(&tag);

        if index == (*args.shell_manager.current_shell).load(Ordering::SeqCst) {
            dict.insert_untagged(" ", "X".to_string());
        } else {
            dict.insert_untagged(" ", " ".to_string());
        }
        dict.insert_untagged("name", shell.name());
        dict.insert_untagged("path", shell.path());

        shells_out.push_back(dict.into_value());
    }

    Ok(shells_out.into())
}
