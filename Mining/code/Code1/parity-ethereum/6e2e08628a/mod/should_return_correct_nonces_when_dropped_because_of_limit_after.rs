fn should_return_correct_nonces_when_dropped_because_of_limit() {
	// given
	let txq = TransactionQueue::new(
		txpool::Options {
			max_count: 3,
			max_per_sender: 1,
			max_mem_usage: 50
		},
		verifier::Options {
			minimal_gas_price: 1.into(),
			block_gas_limit: 1_000_000.into(),
			tx_gas_limit: 1_000_000.into(),
		},
		PrioritizationStrategy::GasPriceOnly,
	);
	let (tx1, tx2) = Tx::gas_price(2).signed_pair();
	let sender = tx1.sender();
	let nonce = tx1.nonce;

	// when
	let r1= txq.import(TestClient::new(), vec![tx1].local());
	let r2= txq.import(TestClient::new(), vec![tx2].local());
	assert_eq!(r1, vec![Ok(())]);
	assert_eq!(r2, vec![Err(transaction::Error::LimitReached)]);
	assert_eq!(txq.status().status.transaction_count, 1);

	// then
	assert_eq!(txq.next_nonce(TestClient::new(), &sender), Some(nonce + 1.into()));

	// when
	let tx1 = Tx::gas_price(2).signed();
	let tx2 = Tx::gas_price(2).signed();
	let tx3 = Tx::gas_price(1).signed();
	let tx4 = Tx::gas_price(3).signed();
	let res = txq.import(TestClient::new(), vec![tx1, tx2].local());
	let res2 = txq.import(TestClient::new(), vec![tx3, tx4].local());

	// then
	assert_eq!(res, vec![Ok(()), Ok(())]);
	assert_eq!(res2, vec![Err(transaction::Error::LimitReached), Ok(())]);
	assert_eq!(txq.status().status.transaction_count, 3);
	// First inserted transacton got dropped because of limit
	assert_eq!(txq.next_nonce(TestClient::new(), &sender), None);
}
