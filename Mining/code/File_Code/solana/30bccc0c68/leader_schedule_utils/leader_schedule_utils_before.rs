use crate::leader_schedule::LeaderSchedule;
use crate::staking_utils;
use solana_runtime::bank::Bank;
use solana_sdk::pubkey::Pubkey;

/// Return the leader schedule for the given epoch.
fn leader_schedule(epoch_height: u64, bank: &Bank) -> LeaderSchedule {
    let stakes = staking_utils::staked_nodes_at_epoch(bank, epoch_height);
    let mut seed = [0u8; 32];
    seed[0..8].copy_from_slice(&epoch_height.to_le_bytes());
    let stakes: Vec<_> = stakes.into_iter().collect();
    LeaderSchedule::new(&stakes, seed, bank.slots_per_epoch())
}

/// Return the leader for the slot at the slot_index and epoch_height returned
/// by the given function.
fn slot_leader_by<F>(bank: &Bank, get_slot_index: F) -> Pubkey
where
    F: Fn(u64, u64, u64) -> (u64, u64),
{
    let (slot_index, epoch_height) = get_slot_index(
        bank.slot_index(),
        bank.epoch_height(),
        bank.slots_per_epoch(),
    );
    let leader_schedule = leader_schedule(epoch_height, bank);
    leader_schedule[slot_index as usize]
}

/// Return the leader for the current slot.
pub fn slot_leader(bank: &Bank) -> Pubkey {
    slot_leader_by(bank, |slot_index, epoch_height, _| {
        (slot_index, epoch_height)
    })
}

/// Return the leader for the given slot.
pub fn slot_leader_at(slot: u64, bank: &Bank) -> Pubkey {
    let epoch = slot / bank.slots_per_epoch();
    slot_leader_by(bank, |_, _, _| (slot, epoch))
}

/// Return the epoch height and slot index of the slot before the current slot.
fn prev_slot_leader_index(slot_index: u64, epoch_height: u64, slots_per_epoch: u64) -> (u64, u64) {
    if epoch_height == 0 && slot_index == 0 {
        return (0, 0);
    }

    if slot_index == 0 {
        (slots_per_epoch - 1, epoch_height - 1)
    } else {
        (slot_index - 1, epoch_height)
    }
}

/// Return the slot_index and epoch height of the slot following the current slot.
fn next_slot_leader_index(slot_index: u64, epoch_height: u64, slots_per_epoch: u64) -> (u64, u64) {
    if slot_index + 1 == slots_per_epoch {
        (0, epoch_height + 1)
    } else {
        (slot_index + 1, epoch_height)
    }
}

/// Return the leader for the slot before the current slot.
pub fn prev_slot_leader(bank: &Bank) -> Pubkey {
    slot_leader_by(bank, prev_slot_leader_index)
}

/// Return the leader for the slot following the current slot.
pub fn next_slot_leader(bank: &Bank) -> Pubkey {
    slot_leader_by(bank, next_slot_leader_index)
}

// Returns the number of ticks remaining from the specified tick_height to the end of the
// slot implied by the tick_height
pub fn num_ticks_left_in_slot(bank: &Bank, tick_height: u64) -> u64 {
    bank.ticks_per_slot() - tick_height % bank.ticks_per_slot() - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::staking_utils;
    use solana_sdk::genesis_block::GenesisBlock;
    use solana_sdk::signature::{Keypair, KeypairUtil};

    #[test]
    fn test_leader_schedule_via_bank() {
        let pubkey = Keypair::new().pubkey();
        let (genesis_block, _mint_keypair) = GenesisBlock::new_with_leader(2, pubkey, 2);
        let bank = Bank::new(&genesis_block);

        let ids_and_stakes: Vec<_> = staking_utils::staked_nodes(&bank).into_iter().collect();
        let seed = [0u8; 32];
        let leader_schedule =
            LeaderSchedule::new(&ids_and_stakes, seed, genesis_block.slots_per_epoch);

        assert_eq!(leader_schedule[0], pubkey);
        assert_eq!(leader_schedule[1], pubkey);
        assert_eq!(leader_schedule[2], pubkey);
    }

    #[test]
    fn test_leader_scheduler1_basic() {
        let pubkey = Keypair::new().pubkey();
        let genesis_block = GenesisBlock::new_with_leader(2, pubkey, 2).0;
        let bank = Bank::new(&genesis_block);
        assert_eq!(slot_leader(&bank), pubkey);
    }

    #[test]
    fn test_leader_scheduler1_prev_slot_leader_index() {
        assert_eq!(prev_slot_leader_index(0, 0, 2), (0, 0));
        assert_eq!(prev_slot_leader_index(1, 0, 2), (0, 0));
        assert_eq!(prev_slot_leader_index(0, 1, 2), (1, 0));
    }

    #[test]
    fn test_leader_scheduler1_next_slot_leader_index() {
        assert_eq!(next_slot_leader_index(0, 0, 2), (1, 0));
        assert_eq!(next_slot_leader_index(1, 0, 2), (0, 1));
    }
}
