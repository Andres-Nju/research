	pub fn new(sstore: Box<SecretStore>, settings: AccountProviderSettings) -> Self {
		let mut hardware_store = None;
		if settings.enable_hardware_wallets {
			match HardwareWalletManager::new() {
				Ok(manager) => {
					manager.set_key_path(if settings.hardware_wallet_classic_key { KeyPath::EthereumClassic } else { KeyPath::Ethereum });
					hardware_store = Some(manager)
				},
				Err(e) => warn!("Error initializing hardware wallets: {}", e),
			}
		}
		AccountProvider {
			unlocked: RwLock::new(HashMap::new()),
			address_book: RwLock::new(AddressBook::new(&sstore.local_path())),
			dapps_settings: RwLock::new(DappsSettingsStore::new(&sstore.local_path())),
			sstore: sstore,
			transient_sstore: transient_sstore(),
			hardware_store: hardware_store,
		}
	}
