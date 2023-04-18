    fn test_sdk_serialize() {
        let keypair = &GenKeys::new([0u8; 32]).gen_n_keypairs(1)[0];
        let to = Pubkey::new(&[
            1, 1, 1, 4, 5, 6, 7, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 8, 7, 6, 5, 4,
            1, 1, 1,
        ]);
        let contract = Pubkey::new(&[
            2, 2, 2, 4, 5, 6, 7, 8, 9, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 9, 8, 7, 6, 5, 4,
            2, 2, 2,
        ]);
        let date =
            DateTime::<Utc>::from_utc(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11), Utc);
        let date_iso8601 = "2016-07-08T09:10:11Z";

        let tx = Transaction::budget_new(&keypair, to, 192, Hash::default());
        assert_eq!(
            tx.userdata,
            vec![
                0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0,
                0, 0, 1, 1, 1, 4, 5, 6, 7, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 8, 7,
                6, 5, 4, 1, 1, 1
            ]
        );

        let tx =
            Transaction::budget_new_on_date(&keypair, to, contract, date, 192, Hash::default());
        assert_eq!(
            tx.userdata,
            vec![
                0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0,
                0, 0, 0, 0, 0, 50, 48, 49, 54, 45, 48, 55, 45, 48, 56, 84, 48, 57, 58, 49, 48, 58,
                49, 49, 90, 32, 253, 186, 201, 177, 11, 117, 135, 187, 167, 181, 188, 22, 59, 206,
                105, 231, 150, 215, 30, 78, 212, 76, 16, 252, 180, 72, 134, 137, 247, 161, 68, 192,
                0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 4, 5, 6, 7, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 9, 8, 7, 6, 5, 4, 1, 1, 1, 1, 0, 0, 0, 32, 253, 186, 201, 177, 11, 117, 135,
                187, 167, 181, 188, 22, 59, 206, 105, 231, 150, 215, 30, 78, 212, 76, 16, 252, 180,
                72, 134, 137, 247, 161, 68, 192, 0, 0, 0, 0, 0, 0, 0, 32, 253, 186, 201, 177, 11,
                117, 135, 187, 167, 181, 188, 22, 59, 206, 105, 231, 150, 215, 30, 78, 212, 76, 16,
                252, 180, 72, 134, 137, 247, 161, 68
            ]
        );

        // ApplyTimestamp(date)
        let tx = Transaction::budget_new_timestamp(
            &keypair,
            keypair.pubkey(),
            to,
            date,
            Hash::default(),
        );
        let mut expected_userdata = vec![1, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0];
        expected_userdata.extend(date_iso8601.as_bytes());
        assert_eq!(tx.userdata, expected_userdata);

        // ApplySignature
        let tx = Transaction::budget_new_signature(&keypair, keypair.pubkey(), to, Hash::default());
        assert_eq!(tx.userdata, vec![2, 0, 0, 0]);
    }
