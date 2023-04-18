    pub fn process_entry_crossing(
        _storage_results: &Arc<RwLock<StorageResults>>,
        _storage_keys: &Arc<RwLock<StorageKeys>>,
        keypair: &Arc<Keypair>,
        _ledger_path: &str,
        entry_id: Hash,
        entry_height: u64,
    ) -> Result<()> {
        let mut seed = [0u8; 32];
        let signature = keypair.sign(&entry_id.as_ref());

        seed.copy_from_slice(&signature.as_ref()[..32]);

        let mut rng = ChaChaRng::from_seed(seed);

        // Regenerate the answers
        let num_slices = (entry_height / ENTRIES_PER_SLICE) as usize;
        if num_slices == 0 {
            info!("Ledger has 0 slices!");
            return Ok(());
        }
        // TODO: what if the validator does not have this slice
        let slice = signature.as_ref()[0] as usize % num_slices;

        debug!(
            "storage verifying: slice: {} identities: {}",
            slice, NUM_IDENTITIES,
        );

        let mut samples = vec![];
        for _ in 0..NUM_SAMPLES {
            samples.push(rng.gen_range(0, 10));
        }
        debug!("generated samples: {:?}", samples);
        // TODO: cuda required to generate the reference values
        // but if it is missing, then we need to take care not to
        // process storage mining results.
        #[cfg(feature = "cuda")]
        {
            let mut storage_results = _storage_results.write().unwrap();

            // Lock the keys, since this is the IV memory,
            // it will be updated in-place by the encryption.
            // Should be overwritten by the vote signatures which replace the
            // key values by the time it runs again.
            let mut storage_keys = _storage_keys.write().unwrap();

            match chacha_cbc_encrypt_file_many_keys(
                _ledger_path,
                slice as u64,
                &mut storage_keys,
                &samples,
            ) {
                Ok(hashes) => {
                    debug!("Success! encrypted ledger slice: {}", slice);
                    storage_results.copy_from_slice(&hashes);
                }
                Err(e) => {
                    info!("error encrypting file: {:?}", e);
                    Err(e)?;
                }
            }
        }
        // TODO: bundle up mining submissions from replicators
        // and submit them in a tx to the leader to get reward.
        Ok(())
    }
