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
                Err(err) => {
                    trace!("mapping {:?} to GenericFailure", err);
                    RpcSignatureStatus::GenericFailure
                }
            },
        )
    }
