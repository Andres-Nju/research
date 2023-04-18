    pub fn get_signature_status_with_commitment(
        &self,
        signature: &Signature,
        commitment_config: CommitmentConfig,
    ) -> ClientResult<Option<transaction::Result<()>>> {
        let signature_status = self.client.send(
            &RpcRequest::GetSignatureStatus,
            json!([[signature.to_string()], commitment_config]),
            5,
        )?;
        let result: Response<Vec<Option<TransactionStatus>>> =
            serde_json::from_value(signature_status).unwrap();
        Ok(result.value[0]
            .clone()
            .map(|status_meta| status_meta.status))
    }
