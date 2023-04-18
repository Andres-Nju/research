fn deserialize_bs58_transaction(bs58_transaction: String) -> Result<(Vec<u8>, Transaction)> {
    let wire_transaction = bs58::decode(bs58_transaction)
        .into_vec()
        .map_err(|e| Error::invalid_params(format!("{:?}", e)))?;
    if wire_transaction.len() >= PACKET_DATA_SIZE {
        let err = format!(
            "transaction too large: {} bytes (max: {} bytes)",
            wire_transaction.len(),
            PACKET_DATA_SIZE
        );
        info!("{}", err);
        return Err(Error::invalid_params(&err));
    }
    bincode::options()
        .with_limit(PACKET_DATA_SIZE as u64)
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .deserialize_from(&wire_transaction[..])
        .map_err(|err| {
            info!("transaction deserialize error: {:?}", err);
            Error::invalid_params(&err.to_string())
        })
        .map(|transaction| (wire_transaction, transaction))
}
