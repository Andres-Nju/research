	pub fn iterator_from(&mut self, pos: u64) -> io::Result<FileIterator> {
		let start = std::cmp::min(self.len, pos * 256);
		let mut buf_reader = io::BufReader::new(&self.file);
		buf_reader.seek(SeekFrom::Start(start))?;

		let iter = FileIterator {
			file: buf_reader,
		};

		Ok(iter)
	}
