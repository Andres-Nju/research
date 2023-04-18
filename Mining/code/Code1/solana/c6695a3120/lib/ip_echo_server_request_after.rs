fn ip_echo_server_request(
    ip_echo_server_addr: &SocketAddr,
    msg: IpEchoServerMessage,
) -> Result<IpAddr, String> {
    let mut data = Vec::new();

    let timeout = Duration::new(5, 0);
    TcpStream::connect_timeout(ip_echo_server_addr, timeout)
        .and_then(|mut stream| {
            let msg = bincode::serialize(&msg).expect("serialize IpEchoServerMessage");
            // Start with 4 null bytes to avoid looking like an HTTP GET/POST request
            stream.write_all(&[0; 4])?;

            stream.write_all(&msg)?;

            // Send a '\n' to make this request look HTTP-ish and tickle an error response back from an HTTP server
            stream.write_all(b"\n")?;
            stream.shutdown(std::net::Shutdown::Write)?;
            stream
                .set_read_timeout(Some(Duration::new(10, 0)))
                .expect("set_read_timeout");
            stream.read_to_end(&mut data)
        })
        .and_then(|_| {
            // It's common for users to accidentally confuse the validator's gossip port and JSON
            // RPC port.  Attempt to detect when this occurs by looking for the standard HTTP
            // response header and provide the user with a helpful error message
            if data.len() < 4 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Response too short, received {} bytes", data.len()),
                ));
            }

            let response_header: String = data[0..4].iter().map(|b| *b as char).collect();
            if response_header != "\0\0\0\0" {
                if response_header == "HTTP" {
                    let http_response = data.iter().map(|b| *b as char).collect::<String>();
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Invalid gossip entrypoint. {} looks to be an HTTP port: {}",
                            ip_echo_server_addr, http_response
                        ),
                    ));
                }
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "Invalid gossip entrypoint. {} provided an invalid response header: '{}'",
                        ip_echo_server_addr, response_header
                    ),
                ));
            }

            bincode::deserialize(&data[4..]).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to deserialize: {:?}", err),
                )
            })
        })
        .map_err(|err| err.to_string())
}
