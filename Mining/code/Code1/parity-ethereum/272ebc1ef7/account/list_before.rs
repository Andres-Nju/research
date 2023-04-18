fn list(list_cmd: ListAccounts) -> Result<String, String> {
	let dir = Box::new(keys_dir(list_cmd.path, list_cmd.spec)?);
	let secret_store = Box::new(secret_store(dir, None)?);
	let acc_provider = AccountProvider::new(secret_store, AccountProviderSettings::default());
	let accounts = acc_provider.accounts().map_err(|e| format!("{}", e))?;
	let result = accounts.into_iter()
		.map(|a| format!("0x{:?}", a))
		.collect::<Vec<String>>()
		.join("\n");

	Ok(result)
}
