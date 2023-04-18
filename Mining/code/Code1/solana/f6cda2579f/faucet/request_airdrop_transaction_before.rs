pub fn request_airdrop_transaction(
    faucet_addr: &SocketAddr,
    id: &Pubkey,
    lamports: u64,
    blockhash: Hash,
) -> Result<Transaction, Error> {
    info!(
        "request_airdrop_transaction: faucet_addr={} id={} lamports={} blockhash={}",
        faucet_addr, id, lamports, blockhash
    );

    let mut stream = TcpStream::connect_timeout(faucet_addr, Duration::new(3, 0))?;
    stream.set_read_timeout(Some(Duration::new(10, 0)))?;
    let req = FaucetRequest::GetAirdrop {
        lamports,
        blockhash,
        to: *id,
    };
    let req = serialize(&req).expect("serialize faucet request");
    stream.write_all(&req)?;

    // Read length of transaction
    let mut buffer = [0; 2];
    stream.read_exact(&mut buffer).map_err(|err| {
        info!(
            "request_airdrop_transaction: buffer length read_exact error: {:?}",
            err
        );
        Error::new(ErrorKind::Other, "Airdrop failed")
    })?;
    let transaction_length = LittleEndian::read_u16(&buffer) as usize;
    if transaction_length >= PACKET_DATA_SIZE || transaction_length == 0 {
        return Err(Error::new(
            ErrorKind::Other,
            format!(
                "request_airdrop_transaction: invalid transaction_length from faucet: {}",
                transaction_length
            ),
        ));
    }

    // Read the transaction
    let mut buffer = Vec::new();
    buffer.resize(transaction_length, 0);
    stream.read_exact(&mut buffer).map_err(|err| {
        info!(
            "request_airdrop_transaction: buffer read_exact error: {:?}",
            err
        );
        Error::new(ErrorKind::Other, "Airdrop failed")
    })?;

    let transaction: Transaction = deserialize(&buffer).map_err(|err| {
        Error::new(
            ErrorKind::Other,
            format!("request_airdrop_transaction deserialize failure: {:?}", err),
        )
    })?;
    Ok(transaction)
}
