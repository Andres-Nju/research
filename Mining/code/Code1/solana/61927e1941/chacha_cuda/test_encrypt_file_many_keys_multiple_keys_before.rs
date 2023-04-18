    fn test_encrypt_file_many_keys_multiple_keys() {
        use logger;
        logger::setup();

        let entries = make_tiny_test_entries(32);
        let ledger_dir = "test_encrypt_file_many_keys_multiple";
        let ledger_path = get_tmp_ledger_path(ledger_dir);
        {
            let mut writer = LedgerWriter::open(&ledger_path, true).unwrap();
            writer.write_entries(entries.clone()).unwrap();
        }

        let out_path = Path::new("test_chacha_encrypt_file_many_keys_multiple_output.txt.enc");

        let samples = [0, 1, 3, 4, 5, 150];
        let mut ivecs = Vec::new();
        let mut ref_hashes: Vec<Hash> = vec![];
        for i in 0..2 {
            let mut ivec = hex!(
                "abc123abc123abc123abc123abc123abc123abababababababababababababab
                                 abc123abc123abc123abc123abc123abc123abababababababababababababab"
            );
            ivec[0] = i;
            ivecs.extend(ivec.clone().iter());
            assert!(
                chacha_cbc_encrypt_file(
                    &Path::new(&ledger_path).join(LEDGER_DATA_FILE),
                    out_path,
                    &mut ivec,
                ).is_ok()
            );

            ref_hashes.push(sample_file(&out_path, &samples).unwrap());
            info!(
                "ivec: {:?} hash: {:?} ivecs: {:?}",
                ivec.to_vec(),
                ref_hashes.last(),
                ivecs
            );
        }

        let hashes =
            chacha_cbc_encrypt_file_many_keys(&ledger_path, 0, &mut ivecs, &samples).unwrap();

        assert_eq!(hashes, ref_hashes);

        let _ignored = remove_dir_all(&ledger_path);
        let _ignored = remove_file(out_path);
    }
