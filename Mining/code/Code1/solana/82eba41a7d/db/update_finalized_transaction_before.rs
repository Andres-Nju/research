pub fn update_finalized_transaction(
    db: &mut PickleDb,
    signature: &Signature,
    opt_transaction_status: Option<TransactionStatus>,
    last_valid_block_height: u64,
    finalized_block_height: u64,
) -> Result<Option<usize>, Error> {
    if opt_transaction_status.is_none() {
        if finalized_block_height > last_valid_block_height {
            eprintln!(
                "Signature not found {} and blockhash expired. Transaction either dropped or the validator purged the transaction status.",
                signature
            );

            // Don't discard the transaction, because we are not certain the
            // blockhash is expired. Instead, return None to signal that
            // we don't need to wait for confirmations.
            return Ok(None);
        }

        // Return zero to signal the transaction may still be in flight.
        return Ok(Some(0));
    }
    let transaction_status = opt_transaction_status.unwrap();

    if let Some(confirmations) = transaction_status.confirmations {
        // The transaction was found but is not yet finalized.
        return Ok(Some(confirmations));
    }

    if let Some(e) = &transaction_status.err {
        // The transaction was finalized, but execution failed. Drop it.
        eprintln!("Error in transaction with signature {}: {}", signature, e);
        eprintln!("Discarding transaction record");
        db.rem(&signature.to_string())?;
        return Ok(None);
    }

    // Transaction is rooted. Set the finalized date in the database.
    let mut transaction_info = db.get::<TransactionInfo>(&signature.to_string()).unwrap();
    transaction_info.finalized_date = Some(Utc::now());
    db.set(&signature.to_string(), &transaction_info)?;
    Ok(None)
}
