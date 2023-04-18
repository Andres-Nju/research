	pub fn new(size: usize) -> BitVecJournal {
		let extra = if size % 8 > 0  { 1 } else { 0 };
		BitVecJournal {
			elems: vec![0u64; size / 8 + extra],
			journal: HashSet::new(),
		}
	}
