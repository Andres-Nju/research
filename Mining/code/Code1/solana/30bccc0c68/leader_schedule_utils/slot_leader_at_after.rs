pub fn slot_leader_at(slot: u64, bank: &Bank) -> Pubkey {
    slot_leader_by(bank, |_, _, _| {
        (slot % bank.slots_per_epoch(), slot / bank.slots_per_epoch())
    })
}
