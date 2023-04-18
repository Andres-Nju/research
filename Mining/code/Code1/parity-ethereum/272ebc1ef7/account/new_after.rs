fn new(n: NewAccount) -> Result<String, String> {
	let password: String = match n.password_file {
		Some(file) => password_from_file(file)?,
		None => password_prompt()?,
	};

	let dir = Box::new(keys_dir(n.path, n.spec)?);
	let secret_store = Box::new(secret_store(dir, Some(n.iterations))?);
	let acc_provider = AccountProvider::new(secret_store, AccountProviderSettings::default());
	let new_account = acc_provider.new_account(&password).map_err(|e| format!("Could not create new account: {}", e))?;
	Ok(format!("0x{:x}", new_account))
}
