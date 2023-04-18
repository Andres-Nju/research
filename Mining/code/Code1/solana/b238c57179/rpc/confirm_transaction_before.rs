        fn confirm_transaction(&self, Self::Metadata, String) -> Result<bool>;

        #[rpc(meta, name = "getAccountInfo")]
        fn get_account_info(&self, Self::Metadata, String) -> Result<Account>;

        #[rpc(meta, name = "getBalance")]
        fn get_balance(&self, Self::Metadata, String) -> Result<i64>;

        #[rpc(meta, name = "getFinality")]
        fn get_finality(&self, Self::Metadata) -> Result<usize>;

        #[rpc(meta, name = "getLastId")]
        fn get_last_id(&self, Self::Metadata) -> Result<String>;

        #[rpc(meta, name = "getSignatureStatus")]
        fn get_signature_status(&self, Self::Metadata, String) -> Result<RpcSignatureStatus>;

        #[rpc(meta, name = "getTransactionCount")]
        fn get_transaction_count(&self, Self::Metadata) -> Result<u64>;

        #[rpc(meta, name= "requestAirdrop")]
        fn request_airdrop(&self, Self::Metadata, String, u64) -> Result<String>;

        #[rpc(meta, name = "sendTransaction")]
        fn send_transaction(&self, Self::Metadata, Vec<u8>) -> Result<String>;
    }
}

pub struct RpcSolImpl;
impl RpcSol for RpcSolImpl {
    type Metadata = Meta;

    fn confirm_transaction(&self, meta: Self::Metadata, id: String) -> Result<bool> {
        self.get_signature_status(meta, id)
            .map(|status| status == RpcSignatureStatus::Confirmed)
    }

    fn get_account_info(&self, meta: Self::Metadata, id: String) -> Result<Account> {
        let pubkey_vec = bs58::decode(id)
            .into_vec()
            .map_err(|_| Error::invalid_request())?;
        if pubkey_vec.len() != mem::size_of::<Pubkey>() {
            return Err(Error::invalid_request());
        }
        let pubkey = Pubkey::new(&pubkey_vec);
        meta.request_processor.get_account_info(pubkey)
    }
    fn get_balance(&self, meta: Self::Metadata, id: String) -> Result<i64> {
        let pubkey_vec = bs58::decode(id)
            .into_vec()
            .map_err(|_| Error::invalid_request())?;
        if pubkey_vec.len() != mem::size_of::<Pubkey>() {
            return Err(Error::invalid_request());
        }
        let pubkey = Pubkey::new(&pubkey_vec);
        meta.request_processor.get_balance(pubkey)
    }
    fn get_finality(&self, meta: Self::Metadata) -> Result<usize> {
        meta.request_processor.get_finality()
    }
    fn get_last_id(&self, meta: Self::Metadata) -> Result<String> {
        meta.request_processor.get_last_id()
    }
    fn get_signature_status(&self, meta: Self::Metadata, id: String) -> Result<RpcSignatureStatus> {
        let signature_vec = bs58::decode(id)
            .into_vec()
            .map_err(|_| Error::invalid_request())?;
        if signature_vec.len() != mem::size_of::<Signature>() {
            return Err(Error::invalid_request());
        }
        let signature = Signature::new(&signature_vec);
        Ok(
            match meta.request_processor.get_signature_status(signature) {
                Ok(_) => RpcSignatureStatus::Confirmed,
                Err(BankError::ProgramRuntimeError) => RpcSignatureStatus::ProgramRuntimeError,
                Err(BankError::SignatureNotFound) => RpcSignatureStatus::SignatureNotFound,
                Err(_) => RpcSignatureStatus::GenericFailure,
            },
        )
    }
    fn get_transaction_count(&self, meta: Self::Metadata) -> Result<u64> {
        meta.request_processor.get_transaction_count()
    }
    fn request_airdrop(&self, meta: Self::Metadata, id: String, tokens: u64) -> Result<String> {
        let pubkey_vec = bs58::decode(id)
            .into_vec()
            .map_err(|_| Error::invalid_request())?;
        if pubkey_vec.len() != mem::size_of::<Pubkey>() {
            return Err(Error::invalid_request());
        }
        let pubkey = Pubkey::new(&pubkey_vec);
        let signature = request_airdrop(&meta.drone_addr, &pubkey, tokens)
            .map_err(|_| Error::internal_error())?;
        let now = Instant::now();
        let mut signature_status;
        loop {
            signature_status = meta.request_processor.get_signature_status(signature);

            if signature_status.is_ok() {
                return Ok(bs58::encode(signature).into_string());
            } else if now.elapsed().as_secs() > 5 {
                return Err(Error::internal_error());
            }
            sleep(Duration::from_millis(100));
        }
    }
    fn send_transaction(&self, meta: Self::Metadata, data: Vec<u8>) -> Result<String> {
        let tx: Transaction = deserialize(&data).map_err(|err| {
            debug!("send_transaction: deserialize error: {:?}", err);
            Error::invalid_request()
        })?;
        let transactions_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        transactions_socket
            .send_to(&data, &meta.transactions_addr)
            .map_err(|err| {
                debug!("send_transaction: send_to error: {:?}", err);
                Error::internal_error()
            })?;
        Ok(bs58::encode(tx.signature).into_string())
    }
}
