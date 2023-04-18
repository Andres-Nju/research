    fn test_loading_rust_analyzer() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let (db, roots) = BatchDatabase::load_cargo(path).unwrap();
        let mut n_crates = 0;
        for root in roots {
            for _krate in Crate::source_root_crates(&db, root) {
                n_crates += 1;
            }
        }

        // RA has quite a few crates, but the exact count doesn't matter
        assert!(n_crates > 20);
    }
