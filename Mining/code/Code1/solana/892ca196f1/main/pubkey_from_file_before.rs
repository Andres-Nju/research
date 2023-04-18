fn pubkey_from_file(key_file: &str) -> Result<Pubkey, Box<dyn error::Error>> {
    read_pubkey_file(key_file)
        .or_else(|_| read_keypair_file(key_file).map(|keypair| keypair.pubkey()))
}
