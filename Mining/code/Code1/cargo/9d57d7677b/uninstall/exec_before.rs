pub fn exec(config: &mut Config, args: &ArgMatches) -> CliResult {
    let root = args.value_of("root");
    let specs = args.values_of("spec").unwrap_or_default().collect();
    ops::uninstall(root, specs, &values(args, "bin"), config)?;
    Ok(())
}
