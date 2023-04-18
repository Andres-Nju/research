File_Code/parity-ethereum/9e5e57fdcd/migration/migration_after.rs --- Rust
244                 try!(consolidate_database(legacy::blocks_database_path(path), db_path.clone(), client::DB_COL_BODIES, Extract::Header, &compaction_profi 244                 try!(consolidate_database(legacy::blocks_database_path(path), db_path.clone(), client::DB_COL_BODIES, Extract::Body, &compaction_profile
    le));                                                                                                                                                        ));

