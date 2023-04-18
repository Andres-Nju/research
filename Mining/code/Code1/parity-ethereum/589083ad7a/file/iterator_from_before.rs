	pub fn iterator_from(&mut self, pos: u64) -> io::Result<FileIterator> {
		let mut buf_reader = io::BufReader::new(&self.file);
		buf_reader.seek(SeekFrom::Start(pos * 256))?;

		let iter = FileIterator {
			file: buf_reader,
		};

		Ok(iter)
	}
