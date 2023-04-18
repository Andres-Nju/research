    fn test_process_cross_program() {
        #[derive(Serialize, Deserialize)]
        enum MockInstruction {
            NoopSuccess,
            NoopFail,
            ModifyOwned,
            ModifyNotOwned,
        }

        fn mock_process_instruction(
            program_id: &Pubkey,
            keyed_accounts: &[KeyedAccount],
            data: &[u8],
        ) -> Result<(), InstructionError> {
            assert_eq!(*program_id, keyed_accounts[0].owner()?);
            assert_ne!(
                keyed_accounts[1].owner()?,
                *keyed_accounts[0].unsigned_key()
            );

            if let Ok(instruction) = bincode::deserialize(data) {
                match instruction {
                    MockInstruction::NoopSuccess => (),
                    MockInstruction::NoopFail => return Err(InstructionError::GenericError),
                    MockInstruction::ModifyOwned => {
                        keyed_accounts[0].try_account_ref_mut()?.data[0] = 1
                    }
                    MockInstruction::ModifyNotOwned => {
                        keyed_accounts[1].try_account_ref_mut()?.data[0] = 1
                    }
                }
            } else {
                return Err(InstructionError::InvalidInstructionData);
            }
            Ok(())
        }

        let caller_program_id = Pubkey::new_rand();
        let callee_program_id = Pubkey::new_rand();
        let mut message_processor = MessageProcessor::default();
        message_processor.add_program(callee_program_id, mock_process_instruction);

        let mut program_account = Account::new(1, 0, &native_loader::id());
        program_account.executable = true;
        let executable_accounts = vec![(callee_program_id, RefCell::new(program_account))];

        let owned_key = Pubkey::new_rand();
        let owned_account = Account::new(42, 1, &callee_program_id);
        let owned_preaccount = PreAccount::new(&owned_key, &owned_account, false, true);

        let not_owned_key = Pubkey::new_rand();
        let not_owned_account = Account::new(84, 1, &Pubkey::new_rand());
        let not_owned_preaccount = PreAccount::new(&not_owned_key, &not_owned_account, false, true);

        let mut accounts = vec![
            Rc::new(RefCell::new(owned_account)),
            Rc::new(RefCell::new(not_owned_account)),
        ];
        let mut invoke_context = ThisInvokeContext::new(
            &caller_program_id,
            Rent::default(),
            vec![owned_preaccount, not_owned_preaccount],
            vec![],
            None,
        );
        let metas = vec![
            AccountMeta::new(owned_key, false),
            AccountMeta::new(not_owned_key, false),
        ];

        // not owned account modified by the caller (before the invoke)
        accounts[0].borrow_mut().data[0] = 1;
        let instruction = Instruction::new(
            callee_program_id,
            &MockInstruction::NoopSuccess,
            metas.clone(),
        );
        let message = Message::new(&[instruction], None);
        assert_eq!(
            message_processor.process_cross_program_instruction(
                &message,
                &executable_accounts,
                &accounts,
                &mut invoke_context,
            ),
            Err(InstructionError::ExternalAccountDataModified)
        );
        accounts[0].borrow_mut().data[0] = 0;

        let cases = vec![
            (MockInstruction::NoopSuccess, Ok(())),
            (
                MockInstruction::NoopFail,
                Err(InstructionError::GenericError),
            ),
            (MockInstruction::ModifyOwned, Ok(())),
            (
                MockInstruction::ModifyNotOwned,
                Err(InstructionError::ExternalAccountDataModified),
            ),
        ];

        for case in cases {
            let instruction = Instruction::new(callee_program_id, &case.0, metas.clone());
            let message = Message::new(&[instruction], None);
            assert_eq!(
                message_processor.process_cross_program_instruction(
                    &message,
                    &executable_accounts,
                    &accounts,
                    &mut invoke_context,
                ),
                case.1
            );
        }
    }
