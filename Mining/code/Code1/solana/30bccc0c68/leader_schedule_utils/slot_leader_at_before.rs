pub fn slot_leader_at(slot: u64, bank: &Bank) -> Pubkey {
    let epoch = slot / bank.slots_per_epoch();
    slot_leader_by(bank, |_, _, _| (slot, epoch))
}
