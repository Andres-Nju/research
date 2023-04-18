	fn gas_pricer_config(&self) -> Result<GasPricerConfig, String> {
		fn wei_per_gas(usd_per_tx: f32, usd_per_eth: f32) -> U256 {
			let wei_per_usd: f32 = 1.0e18 / usd_per_eth;
			let gas_per_tx: f32 = 21000.0;
			let wei_per_gas: f32 = wei_per_usd * usd_per_tx / gas_per_tx;
			U256::from_dec_str(&format!("{:.0}", wei_per_gas)).unwrap()
		}

		if let Some(dec) = self.args.arg_gasprice.as_ref() {
			return Ok(GasPricerConfig::Fixed(to_u256(dec)?));
		} else if let Some(dec) = self.args.arg_min_gas_price {
			return Ok(GasPricerConfig::Fixed(U256::from(dec)));
		}

		let usd_per_tx = to_price(&self.args.arg_usd_per_tx)?;
		if "auto" == self.args.arg_usd_per_eth.as_str() {
			// Just a very rough estimate to avoid accepting
			// ZGP transactions before the price is fetched
			// if user does not want it.
			let last_known_usd_per_eth = 10.0;
			return Ok(GasPricerConfig::Calibrated {
				initial_minimum: wei_per_gas(usd_per_tx, last_known_usd_per_eth),
				usd_per_tx: usd_per_tx,
				recalibration_period: to_duration(self.args.arg_price_update_period.as_str())?,
			});
		}

		let usd_per_eth = to_price(&self.args.arg_usd_per_eth)?;
		let wei_per_gas = wei_per_gas(usd_per_tx, usd_per_eth);

		info!(
			"Using a fixed conversion rate of Ξ1 = {} ({} wei/gas)",
			Colour::White.bold().paint(format!("US${:.2}", usd_per_eth)),
			Colour::Yellow.bold().paint(format!("{}", wei_per_gas))
		);

		Ok(GasPricerConfig::Fixed(wei_per_gas))
	}
