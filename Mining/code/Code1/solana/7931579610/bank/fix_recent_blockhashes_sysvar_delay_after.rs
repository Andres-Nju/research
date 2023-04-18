    fn fix_recent_blockhashes_sysvar_delay(&self) -> bool {
        let activation_slot = match self.operating_mode() {
            OperatingMode::Development => 0,
            OperatingMode::Preview => Slot::MAX / 2,
            OperatingMode::Stable => Slot::MAX / 2,
        };

        self.slot() >= activation_slot
    }
