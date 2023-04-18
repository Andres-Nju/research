    fn test_max_hashes() {
        solana_logger::setup();
        use std::path::PathBuf;
        use tempfile::TempDir;
        let keypair = Keypair::new();

        let contact_info = ContactInfo::new_localhost(&keypair.pubkey(), 0);
        let cluster_info = ClusterInfo::new_with_invalid_keypair(contact_info);
        let cluster_info = Arc::new(cluster_info);

        let trusted_validators = HashSet::new();
        let exit = Arc::new(AtomicBool::new(false));
        let mut hashes = vec![];
        for i in 0..MAX_SNAPSHOT_HASHES + 1 {
            let snapshot_links = TempDir::new().unwrap();
            let accounts_package = AccountsPackage {
                hash: hash(&[i as u8]),
                block_height: 100 + i as u64,
                root: 100 + i as u64,
                slot_deltas: vec![],
                snapshot_links,
                tar_output_file: PathBuf::from("."),
                storages: vec![],
                compression: CompressionType::Bzip2,
                snapshot_version: SnapshotVersion::default(),
            };

            AccountsHashVerifier::process_accounts_package(
                accounts_package,
                &cluster_info,
                &Some(trusted_validators.clone()),
                false,
                &None,
                &mut hashes,
                &exit,
                0,
                100,
            );
        }
        cluster_info.flush_push_queue();
        let cluster_hashes = cluster_info
            .get_accounts_hash_for_node(&keypair.pubkey(), |c| c.clone())
            .unwrap();
        info!("{:?}", cluster_hashes);
        assert_eq!(hashes.len(), MAX_SNAPSHOT_HASHES);
        assert_eq!(cluster_hashes.len(), MAX_SNAPSHOT_HASHES);
        assert_eq!(cluster_hashes[0], (101, hash(&[1])));
        assert_eq!(
            cluster_hashes[MAX_SNAPSHOT_HASHES - 1],
            (
                100 + MAX_SNAPSHOT_HASHES as u64,
                hash(&[MAX_SNAPSHOT_HASHES as u8])
            )
        );
    }
