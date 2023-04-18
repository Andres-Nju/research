pub fn to_fat_rlps(
	account_hash: &H256,
	acc: &BasicAccount,
	acct_db: &AccountDB,
	used_code: &mut HashSet<H256>,
	first_chunk_size: usize,
	max_chunk_size: usize,
	p: &RwLock<Progress>,
) -> Result<Vec<Bytes>, Error> {
	let db = &(acct_db as &dyn HashDB<_,_>);
	let db = TrieDB::new(db, &acc.storage_root)?;
	let mut chunks = Vec::new();
	let mut db_iter = db.iter()?;
	let mut target_chunk_size = first_chunk_size;
	let mut account_stream = RlpStream::new_list(2);
	let mut leftover: Option<Vec<u8>> = None;
	loop {
		account_stream.append(account_hash);
		let use_short_version = acc.code_version.is_zero();
		match use_short_version {
			true => { account_stream.begin_list(5); },
			false => { account_stream.begin_list(6); },
		}

		account_stream.append(&acc.nonce)
			.append(&acc.balance);

		// [has_code, code_hash].
		if acc.code_hash == KECCAK_EMPTY {
			account_stream.append(&CodeState::Empty.raw()).append_empty_data();
		} else if used_code.contains(&acc.code_hash) {
			account_stream.append(&CodeState::Hash.raw()).append(&acc.code_hash);
		} else {
			match acct_db.get(&acc.code_hash, hash_db::EMPTY_PREFIX) {
				Some(c) => {
					used_code.insert(acc.code_hash.clone());
					account_stream.append(&CodeState::Inline.raw()).append(&&*c);
				}
				None => {
					warn!("code lookup failed during snapshot");
					account_stream.append(&false).append_empty_data();
				}
			}
		}

		if !use_short_version {
			account_stream.append(&acc.code_version);
		}

		account_stream.begin_unbounded_list();
		if account_stream.len() > target_chunk_size {
			// account does not fit, push an empty record to mark a new chunk
			target_chunk_size = max_chunk_size;
			chunks.push(Vec::new());
		}

		if let Some(pair) = leftover.take() {
			if !account_stream.append_raw_checked(&pair, 1, target_chunk_size) {
				return Err(Error::ChunkTooSmall);
			}
		}

		loop {
			if p.read().abort {
				trace!(target: "snapshot", "to_fat_rlps: aborting snapshot");
				return Err(Error::SnapshotAborted);
			}
			match db_iter.next() {
				Some(Ok((k, v))) => {
					let pair = {
						let mut stream = RlpStream::new_list(2);
						stream.append(&k).append(&&*v);
						stream.drain()
					};
					if !account_stream.append_raw_checked(&pair, 1, target_chunk_size) {
						account_stream.complete_unbounded_list();
						let stream = ::std::mem::replace(&mut account_stream, RlpStream::new_list(2));
						chunks.push(stream.out());
						target_chunk_size = max_chunk_size;
						leftover = Some(pair);
						break;
					}
				},
				Some(Err(e)) => {
					return Err(e.into());
				},
				None => {
					account_stream.complete_unbounded_list();
					let stream = ::std::mem::replace(&mut account_stream, RlpStream::new_list(2));
					chunks.push(stream.out());
					return Ok(chunks);
				}
			}

		}
	}
}
