pub fn migrate_blooms<P: AsRef<Path>>(path: P, config: &DatabaseConfig) -> Result<(), Error> {
	// init
	let db = open_database(&path.as_ref().to_string_lossy(), config)?;

	// possible optimization:
	// pre-allocate space on disk for faster migration

	// iterate over header blooms and insert them in blooms-db
	// Some(3) -> COL_EXTRA
	// 3u8 -> ExtrasIndex::BlocksBlooms
	// 0u8 -> level 0
	let blooms_iterator = db.key_value()
		.iter_from_prefix(Some(3), &[3u8, 0u8])
		.filter(|(key, _)| key.len() == 6)
		.take_while(|(key, _)| {
			key[0] == 3u8 && key[1] == 0u8
		})
		.map(|(key, group)| {
			let number =
				(key[2] as u64) << 24 |
				(key[3] as u64) << 16 |
				(key[4] as u64) << 8 |
				(key[5] as u64);

			let blooms = rlp::decode_list::<Bloom>(&group);
			(number, blooms)
		});

	for (number, blooms) in blooms_iterator {
		db.blooms().insert_blooms(number, blooms.iter())?;
	}

	// iterate over trace blooms and insert them in blooms-db
	// Some(4) -> COL_TRACE
	// 1u8 -> TraceDBIndex::BloomGroups
	// 0u8 -> level 0
	let trace_blooms_iterator = db.key_value()
		.iter_from_prefix(Some(4), &[1u8, 0u8])
		.filter(|(key, _)| key.len() == 6)
		.take_while(|(key, _)| {
			key[0] == 1u8 && key[1] == 0u8
		})
		.map(|(key, group)| {
			let number =
				(key[2] as u64) |
				(key[3] as u64) << 8 |
				(key[4] as u64) << 16 |
				(key[5] as u64) << 24;

			let blooms = rlp::decode_list::<Bloom>(&group);
			(number, blooms)
		});

	for (number, blooms) in trace_blooms_iterator {
		db.trace_blooms().insert_blooms(number, blooms.iter())?;
	}

	Ok(())
}
