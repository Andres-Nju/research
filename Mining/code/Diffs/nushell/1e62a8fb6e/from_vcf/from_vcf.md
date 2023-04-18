File_Code/nushell/1e62a8fb6e/from_vcf/from_vcf_after.rs --- Rust
45     let buf_reader = std::io::Cursor::new(input_bytes);                                                                                                   45     let cursor = std::io::Cursor::new(input_bytes);
46     let parser = ical::VcardParser::new(buf_reader);                                                                                                      46     let parser = ical::VcardParser::new(cursor);

