    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn write_vote_account(
            f: &mut fmt::Formatter,
            validator: &CliValidator,
            total_active_stake: u64,
            use_lamports_unit: bool,
            delinquent: bool,
        ) -> fmt::Result {
            fn non_zero_or_dash(v: u64) -> String {
                if v == 0 {
                    "-".into()
                } else {
                    format!("{}", v)
                }
            }

            writeln!(
                f,
                "{} {:<44}  {:<44}  {:>9}%   {:>8}  {:>10}  {:>10}  {}",
                if delinquent {
                    WARNING.to_string()
                } else {
                    " ".to_string()
                },
                validator.identity_pubkey,
                validator.vote_account_pubkey,
                validator.commission,
                non_zero_or_dash(validator.last_vote),
                non_zero_or_dash(validator.root_slot),
                validator.credits,
                if validator.activated_stake > 0 {
                    format!(
                        "{} ({:.2}%)",
                        build_balance_message(validator.activated_stake, use_lamports_unit, true),
                        100. * validator.activated_stake as f64 / total_active_stake as f64,
                    )
                } else {
                    "-".into()
                },
            )
        }
        writeln_name_value(
            f,
            "Active Stake:",
            &build_balance_message(self.total_active_stake, self.use_lamports_unit, true),
        )?;
        if self.total_deliquent_stake > 0 {
            writeln_name_value(
                f,
                "Current Stake:",
                &format!(
                    "{} ({:0.2}%)",
                    &build_balance_message(self.total_current_stake, self.use_lamports_unit, true),
                    100. * self.total_current_stake as f64 / self.total_active_stake as f64
                ),
            )?;
            writeln_name_value(
                f,
                "Delinquent Stake:",
                &format!(
                    "{} ({:0.2}%)",
                    &build_balance_message(
                        self.total_deliquent_stake,
                        self.use_lamports_unit,
                        true
                    ),
                    100. * self.total_deliquent_stake as f64 / self.total_active_stake as f64
                ),
            )?;
        }
        writeln!(f)?;
        writeln!(
            f,
            "{}",
            style(format!(
                "  {:<44}  {:<44}  {}  {}  {}  {:>10}  {}",
                "Identity Pubkey",
                "Vote Account Pubkey",
                "Commission",
                "Last Vote",
                "Root Block",
                "Credits",
                "Active Stake",
            ))
            .bold()
        )?;
        for validator in &self.current_validators {
            write_vote_account(
                f,
                validator,
                self.total_active_stake,
                self.use_lamports_unit,
                false,
            )?;
        }
        for validator in &self.delinquent_validators {
            write_vote_account(
                f,
                validator,
                self.total_active_stake,
                self.use_lamports_unit,
                true,
            )?;
        }
        Ok(())
    }
