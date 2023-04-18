	pub fn new(
		imported: Vec<H256>,
		invalid: Vec<H256>,
		route: ChainRoute,
		sealed: Vec<H256>,
		proposed: Vec<Bytes>,
		duration: Duration,
		has_more_blocks_to_import: bool,
	) -> NewBlocks {
		NewBlocks {
			imported,
			invalid,
			route,
			sealed,
			proposed,
			duration,
			has_more_blocks_to_import,
		}
	}
