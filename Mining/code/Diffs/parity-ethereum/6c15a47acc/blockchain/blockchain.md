File_Code/parity-ethereum/6c15a47acc/blockchain/blockchain_after.rs --- Rust
197                                 try!(instream.read_exact(&mut bytes[READAHEAD_BYTES..]).map_err(|_| "Error reading from the file/stream."));             197                                 try!(instream.read_exact(&mut bytes[n..]).map_err(|_| "Error reading from the file/stream."));

