fn entrypoint(
    program_id: &Pubkey,
    keyed_accounts: &mut [KeyedAccount],
    tx_data: &[u8],
    tick_height: u64,
) -> Result<(), ProgramError> {
    static INIT: Once = ONCE_INIT;
    INIT.call_once(|| {
        // env_logger can only be initialized once
        env_logger::init();
    });

    if keyed_accounts[0].account.executable {
        let prog = keyed_accounts[0].account.userdata.clone();
        info!("Call BPF program");
        //dump_program(keyed_accounts[0].key, &prog);
        let mut vm = match create_vm(&prog) {
            Ok(vm) => vm,
            Err(e) => {
                warn!("create_vm failed: {}", e);
                return Err(ProgramError::GenericError);
            }
        };
        let mut v =
            serialize_parameters(program_id, &mut keyed_accounts[1..], &tx_data, tick_height);
        match vm.execute_program(v.as_mut_slice()) {
            Ok(status) => {
                if 0 == status {
                    return Err(ProgramError::GenericError);
                }
            }
            Err(e) => {
                warn!("execute_program failed: {}", e);
                return Err(ProgramError::GenericError);
            }
        }
        deserialize_parameters(&mut keyed_accounts[1..], &v);
        info!(
            "BPF program executed {} instructions",
            vm.get_last_instruction_count()
        );
    } else if let Ok(instruction) = deserialize(tx_data) {
        if keyed_accounts[0].signer_key().is_none() {
            warn!("key[0] did not sign the transaction");
            return Err(ProgramError::GenericError);
        }
        match instruction {
            LoaderInstruction::Write { offset, bytes } => {
                let offset = offset as usize;
                let len = bytes.len();
                debug!("Write: offset={} length={}", offset, len);
                if keyed_accounts[0].account.userdata.len() < offset + len {
                    warn!(
                        "Write overflow: {} < {}",
                        keyed_accounts[0].account.userdata.len(),
                        offset + len
                    );
                    return Err(ProgramError::GenericError);
                }
                keyed_accounts[0].account.userdata[offset..offset + len].copy_from_slice(&bytes);
            }
            LoaderInstruction::Finalize => {
                keyed_accounts[0].account.executable = true;
                info!(
                    "Finalize: account {:?}",
                    keyed_accounts[0].signer_key().unwrap()
                );
            }
        }
    } else {
        warn!("Invalid program transaction: {:?}", tx_data);
        return Err(ProgramError::GenericError);
    }
    Ok(())
}
