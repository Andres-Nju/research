    pub fn connect_timeout(_addr: &SocketAddr, _timeout: Duration) -> Result<TcpStream> {
        Err(Error::new(ErrorKind::Other, "TcpStream::connect_timeout not implemented"))
    }