	fn corpus_inaccessible() {
		let mut cache = Cache::new(Default::default(), Duration::from_secs(5 * 3600));

		cache.set_gas_price_corpus(vec![].into());
		assert_eq!(cache.gas_price_corpus(), Some(vec![].into()));

		{
			let corpus_time = &mut cache.corpus.as_mut().unwrap().1;
			*corpus_time = *corpus_time - Duration::from_secs(5 * 3600);
		}
		assert!(cache.gas_price_corpus().is_none());
	}
