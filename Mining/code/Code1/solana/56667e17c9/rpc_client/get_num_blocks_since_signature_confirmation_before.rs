    pub fn get_num_blocks_since_signature_confirmation(
        &self,
        signature: &Signature,
    ) -> ClientResult<usize> {
        let response = self
            .client
            .send(
                &RpcRequest::GetSignatureStatus,
                json!([[signature.to_string()], CommitmentConfig::recent().ok()]),
                1,
            )
            .map_err(|err| err.into_with_command("GetSignatureStatus"))?;
        let result: Response<Vec<Option<TransactionStatus>>> =
            serde_json::from_value(response).unwrap();

        let confirmations = result.value[0]
            .clone()
            .ok_or_else(|| {
                ClientError::new_with_command(
                    ClientErrorKind::Custom("signature not found".to_string()),
                    "GetSignatureStatus",
                )
            })?
            .confirmations
            .unwrap_or(MAX_LOCKOUT_HISTORY + 1);
        Ok(confirmations)
    }
