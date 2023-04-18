pub fn kill_db(cmd: KillBlockchain) -> Result<(), String> {
	let spec = cmd.spec.spec(&cmd.dirs.cache)?;
	let genesis_hash = spec.genesis_header().hash();
	let db_dirs = cmd.dirs.database(genesis_hash, None, spec.data_dir);
	let user_defaults_path = db_dirs.user_defaults_path();
	let mut user_defaults = UserDefaults::load(&user_defaults_path)?;
	let algorithm = cmd.pruning.to_algorithm(&user_defaults);
	let dir = db_dirs.db_path(algorithm);
	fs::remove_dir_all(&dir).map_err(|e| format!("Error removing database: {:?}", e))?;
	user_defaults.is_first_launch = true;
	user_defaults.save(&user_defaults_path)?;
	info!("Database deleted.");
	Ok(())
}
