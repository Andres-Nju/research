pub fn run_faucet(
    faucet: Arc<Mutex<Faucet>>,
    faucet_addr: SocketAddr,
    send_addr: Option<Sender<SocketAddr>>,
) {
    let socket = TcpListener::bind(&faucet_addr).unwrap();
    if let Some(send_addr) = send_addr {
        send_addr.send(socket.local_addr().unwrap()).unwrap();
    }
    info!("Faucet started. Listening on: {}", faucet_addr);
    let done = socket
        .incoming()
        .map_err(|e| debug!("failed to accept socket; error = {:?}", e))
        .for_each(move |socket| {
            let faucet2 = faucet.clone();
            let framed = BytesCodec::new().framed(socket);
            let (writer, reader) = framed.split();

            let processor = reader.and_then(move |bytes| {
                match faucet2.lock().unwrap().process_faucet_request(&bytes) {
                    Ok(response_bytes) => {
                        trace!("Airdrop response_bytes: {:?}", response_bytes.to_vec());
                        Ok(response_bytes)
                    }
                    Err(e) => {
                        info!("Error in request: {:?}", e);
                        Ok(Bytes::from(0u16.to_le_bytes().to_vec()))
                    }
                }
            });
            let server = writer
                .send_all(processor.or_else(|err| {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Faucet response: {:?}", err),
                    ))
                }))
                .then(|_| Ok(()));
            tokio::spawn(server)
        });
    tokio::run(done);
}
