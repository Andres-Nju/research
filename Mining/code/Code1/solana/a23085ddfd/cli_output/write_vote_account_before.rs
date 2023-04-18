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
                "{} {:<44}  {:<44}  {:>9}%   {:>8}  {:>10}  {:>7}  {}",
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
                        100. * validator.activated_stake as f64 / total_active_stake as f64
                    )
                } else {
                    "-".into()
                },
            )
        }
