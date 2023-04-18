    fn reset(&mut self) -> io::Result<bool> {
        // are there any terminals that have color/attrs and not sgr0?
        // Try falling back to sgr, then op
        let cmd = match ["sgr0", "sgr", "op"]
                            .iter()
                            .filter_map(|cap| self.ti.strings.get(*cap))
                            .next() {
            Some(op) => {
                match expand(&op, &[], &mut Variables::new()) {
                    Ok(cmd) => cmd,
                    Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                }
            }
            None => return Ok(false),
        };
        self.out.write_all(&cmd).and(Ok(true))
    }
