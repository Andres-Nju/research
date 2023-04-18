	fn corpus_inaccessible() {
		let duration = Duration::from_secs(20);
		let mut cache = Cache::new(Default::default(), duration.clone());

		cache.set_gas_price_corpus(vec![].into());
		assert_eq!(cache.gas_price_corpus(), Some(vec![].into()));

		{
			let corpus_time = &mut cache.corpus.as_mut().unwrap().1;
			*corpus_time = *corpus_time - duration;
		}
		assert!(cache.gas_price_corpus().is_none());
	}
