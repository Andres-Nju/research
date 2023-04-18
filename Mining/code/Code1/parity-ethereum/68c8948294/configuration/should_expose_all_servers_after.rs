	fn should_expose_all_servers() {
		// given

		// when
		let conf0 = parse(&["parity", "--unsafe-expose"]);

		// then
		assert_eq!(&conf0.network_settings().unwrap().rpc_interface, "0.0.0.0");
		assert_eq!(&conf0.http_config().unwrap().interface, "0.0.0.0");
		assert_eq!(conf0.http_config().unwrap().hosts, None);
		assert_eq!(&conf0.ws_config().unwrap().interface, "0.0.0.0");
		assert_eq!(conf0.ws_config().unwrap().hosts, None);
		assert_eq!(conf0.ws_config().unwrap().origins, None);
		assert_eq!(&conf0.ui_config().interface, "0.0.0.0");
		assert_eq!(conf0.ui_config().hosts, None);
		assert_eq!(&conf0.secretstore_config().unwrap().interface, "0.0.0.0");
		assert_eq!(&conf0.secretstore_config().unwrap().http_interface, "0.0.0.0");
		assert_eq!(&conf0.ipfs_config().interface, "0.0.0.0");
		assert_eq!(conf0.ipfs_config().hosts, None);
	}
