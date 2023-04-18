File_Code/solana/890f29be0c/accounts_db/accounts_db_after.rs --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
    .                                                                                                                                                        10400         let old_written = storage.written_bytes();
10400         storage.accounts.append_accounts(&storable_accounts, 0);                                                                                       10401         storage.accounts.append_accounts(&storable_accounts, 0);
10401         if mark_alive {                                                                                                                                10402         if mark_alive {
10402             // updates 'alive_bytes'                                                                                                                   10403             // updates 'alive_bytes' on the storage
10403             storage.add_account(storage.accounts.len());                                                                                               10404             storage.add_account((storage.written_bytes() - old_written) as usize);

