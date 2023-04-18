fn should_check_status_of_request_when_its_resolved() {
	// given
	let tester = eth_signing();
	let address = Address::random();
	let request = r#"{
		"jsonrpc": "2.0",
		"method": "parity_postSign",
		"params": [
			""#.to_owned() + format!("0x{:x}", address).as_ref() + r#"",
			"0x0000000000000000000000000000000000000000000000000000000000000005"
		],
		"id": 1
	}"#;
	tester.io.handle_request_sync(&request).expect("Sent");
	let sender = tester.signer.take(&1.into()).unwrap();
	tester.signer.request_confirmed(sender, Ok(ConfirmationResponse::Signature(Signature::from_low_u64_be(1))));

	// This is not ideal, but we need to give futures some time to be executed, and they need to run in a separate thread
	thread::sleep(Duration::from_millis(100));

	// when
	let request = r#"{
		"jsonrpc": "2.0",
		"method": "parity_checkRequest",
		"params": ["0x1"],
		"id": 1
	}"#;
	let response = r#"{"jsonrpc":"2.0","result":"0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001","id":1}"#;

	// then
	assert_eq!(tester.io.handle_request_sync(&request), Some(response.to_owned()));
}
