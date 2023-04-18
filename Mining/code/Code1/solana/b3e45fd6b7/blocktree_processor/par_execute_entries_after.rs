fn par_execute_entries(
    bank: &Bank,
    entries: &[(&Entry, LockedAccountsResults<Transaction>)],
) -> Result<()> {
    inc_new_counter_info!("bank-par_execute_entries-count", entries.len());
    let results: Vec<Result<()>> = entries
        .into_par_iter()
        .map(|(e, locked_accounts)| {
            let results = bank.load_execute_and_commit_transactions(
                &e.transactions,
                locked_accounts,
                MAX_RECENT_BLOCKHASHES,
            );
            let mut first_err = None;
            for (r, tx) in results.iter().zip(e.transactions.iter()) {
                if let Err(ref e) = r {
                    if first_err.is_none() {
                        first_err = Some(r.clone());
                    }
                    if !Bank::can_commit(&r) {
                        warn!("Unexpected validator error: {:?}, tx: {:?}", e, tx);
                        datapoint!(
                            "validator_process_entry_error",
                            ("error", format!("error: {:?}, tx: {:?}", e, tx), String)
                        );
                    }
                }
            }
            first_err.unwrap_or(Ok(()))
        })
        .collect();

    first_err(&results)
}
