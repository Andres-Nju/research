    fn test_rpc_get_multiple_accounts() {
        let rpc = RpcHandler::start();
        let bank = rpc.working_bank();

        let non_existent_pubkey = Pubkey::new_unique();
        let pubkey = Pubkey::new_unique();
        let address = pubkey.to_string();
        let data = vec![1, 2, 3, 4, 5];
        let account = AccountSharedData::create(42, data.clone(), Pubkey::default(), false, 0);
        bank.store_account(&pubkey, &account);

        // Test 3 accounts, one empty, one non-existent, and one with data
        let request = create_test_request(
            "getMultipleAccounts",
            Some(json!([[
                rpc.mint_keypair.pubkey().to_string(),
                non_existent_pubkey.to_string(),
                address,
            ]])),
        );
        let result: RpcResponse<Value> = parse_success_result(rpc.handle_request_sync(request));
        let expected = json!([
            {
                "owner": "11111111111111111111111111111111",
                "lamports": TEST_MINT_LAMPORTS,
                "data": ["", "base64"],
                "executable": false,
                "rentEpoch": 0,
                "space": 0,
            },
            null,
            {
                "owner": "11111111111111111111111111111111",
                "lamports": 42,
                "data": [base64::encode(&data), "base64"],
                "executable": false,
                "rentEpoch": 0,
                "space": 5,
            }
        ]);
        assert_eq!(result.value, expected);

        // Test config settings still work with multiple accounts
        let request = create_test_request(
            "getMultipleAccounts",
            Some(json!([
                [
                    rpc.mint_keypair.pubkey().to_string(),
                    non_existent_pubkey.to_string(),
                    address,
                ],
                {"encoding": "base58"},
            ])),
        );
        let result: RpcResponse<Value> = parse_success_result(rpc.handle_request_sync(request));
        let expected = json!([
            {
                "owner": "11111111111111111111111111111111",
                "lamports": TEST_MINT_LAMPORTS,
                "data": ["", "base58"],
                "executable": false,
                "rentEpoch": 0,
                "space": 0,
            },
            null,
            {
                "owner": "11111111111111111111111111111111",
                "lamports": 42,
                "data": [bs58::encode(&data).into_string(), "base58"],
                "executable": false,
                "rentEpoch": 0,
                "space": 5,
            }
        ]);
        assert_eq!(result.value, expected);

        let request = create_test_request(
            "getMultipleAccounts",
            Some(json!([
                [
                    rpc.mint_keypair.pubkey().to_string(),
                    non_existent_pubkey.to_string(),
                    address,
                ],
                {"encoding": "jsonParsed", "dataSlice": {"length": 2, "offset": 1}},
            ])),
        );
        let result: RpcResponse<Value> = parse_success_result(rpc.handle_request_sync(request));
        let expected = json!([
            {
                "owner": "11111111111111111111111111111111",
                "lamports": TEST_MINT_LAMPORTS,
                "data": ["", "base64"],
                "executable": false,
                "rentEpoch": 0,
                "space": 0,
            },
            null,
            {
                "owner": "11111111111111111111111111111111",
                "lamports": 42,
                "data": [base64::encode(&data[1..3]), "base64"],
                "executable": false,
                "rentEpoch": 0,
                "space": 5,
            }
        ]);
        assert_eq!(
            result.value, expected,
            "should use data slice if parsing fails"
        );
    }
