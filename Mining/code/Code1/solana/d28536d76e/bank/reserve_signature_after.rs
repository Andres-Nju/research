    fn reserve_signature(signatures: &mut HashSet<Signature>, sig: &Signature) -> Result<()> {
        if let Some(sig) = signatures.get(sig) {
            return Err(BankError::DuplicateSignature(*sig));
        }
        signatures.insert(*sig);
        Ok(())
    }
