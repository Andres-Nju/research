fn run_final_hash_calc(bank: &Bank, on_halt_store_hash_raw_data_for_debug: bool) {
    bank.force_flush_accounts_cache();
    let can_cached_slot_be_unflushed = false;
    // note that this slot may not be a root
    let _ = bank.verify_bank_hash(VerifyBankHash {
        test_hash_calculation: false,
        can_cached_slot_be_unflushed,
        ignore_mismatch: true,
        require_rooted_bank: false,
        run_in_background: false,
        store_hash_raw_data_for_debug: on_halt_store_hash_raw_data_for_debug,
    });
}
