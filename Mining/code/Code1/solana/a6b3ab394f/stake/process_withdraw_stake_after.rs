pub fn process_withdraw_stake(
    rpc_client: &RpcClient,
    config: &CliConfig,
    stake_account_pubkey: &Pubkey,
    destination_account_pubkey: &Pubkey,
    amount: SpendAmount,
    withdraw_authority: SignerIndex,
    custodian: Option<SignerIndex>,
    sign_only: bool,
    dump_transaction_message: bool,
    blockhash_query: &BlockhashQuery,
    nonce_account: Option<&Pubkey>,
    nonce_authority: SignerIndex,
    memo: Option<&String>,
    seed: Option<&String>,
    fee_payer: SignerIndex,
) -> ProcessResult {
    let withdraw_authority = config.signers[withdraw_authority];
    let custodian = custodian.map(|index| config.signers[index]);

    let stake_account_address = if let Some(seed) = seed {
        Pubkey::create_with_seed(stake_account_pubkey, seed, &stake::program::id())?
    } else {
        *stake_account_pubkey
    };

    let recent_blockhash = blockhash_query.get_blockhash(rpc_client, config.commitment)?;

    let fee_payer = config.signers[fee_payer];
    let nonce_authority = config.signers[nonce_authority];

    let build_message = |lamports| {
        let ixs = vec![stake_instruction::withdraw(
            &stake_account_address,
            &withdraw_authority.pubkey(),
            destination_account_pubkey,
            lamports,
            custodian.map(|signer| signer.pubkey()).as_ref(),
        )]
        .with_memo(memo);

        if let Some(nonce_account) = &nonce_account {
            Message::new_with_nonce(
                ixs,
                Some(&fee_payer.pubkey()),
                nonce_account,
                &nonce_authority.pubkey(),
            )
        } else {
            Message::new(&ixs, Some(&fee_payer.pubkey()))
        }
    };

    let (message, _) = resolve_spend_tx_and_check_account_balances(
        rpc_client,
        sign_only,
        amount,
        &recent_blockhash,
        &stake_account_address,
        &fee_payer.pubkey(),
        build_message,
        config.commitment,
    )?;

    let mut tx = Transaction::new_unsigned(message);

    if sign_only {
        tx.try_partial_sign(&config.signers, recent_blockhash)?;
        return_signers_with_config(
            &tx,
            &config.output_format,
            &ReturnSignersConfig {
                dump_transaction_message,
            },
        )
    } else {
        tx.try_sign(&config.signers, recent_blockhash)?;
        if let Some(nonce_account) = &nonce_account {
            let nonce_account = nonce_utils::get_account_with_commitment(
                rpc_client,
                nonce_account,
                config.commitment,
            )?;
            check_nonce_account(&nonce_account, &nonce_authority.pubkey(), &recent_blockhash)?;
        }
        check_account_for_fee_with_commitment(
            rpc_client,
            &tx.message.account_keys[0],
            &tx.message,
            config.commitment,
        )?;
        let result = rpc_client.send_and_confirm_transaction_with_spinner(&tx);
        log_instruction_custom_error::<StakeError>(result, config)
    }
}
