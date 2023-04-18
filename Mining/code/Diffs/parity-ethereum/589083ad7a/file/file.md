File_Code/parity-ethereum/589083ad7a/file/file_after.rs --- Rust
 .                                                                                                                                                           95                 let start = std::cmp::min(self.len, pos * 256);
95                 let mut buf_reader = io::BufReader::new(&self.file);                                                                                      96                 let mut buf_reader = io::BufReader::new(&self.file);
96                 buf_reader.seek(SeekFrom::Start(pos * 256))?;                                                                                             97                 buf_reader.seek(SeekFrom::Start(start))?;

