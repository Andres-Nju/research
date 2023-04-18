//! The `timing` module provides std::time utility functions.
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

// The default tick rate that the cluster attempts to achieve.  Note that the actual tick
// rate at any given time should be expected to drift
pub const DEFAULT_NUM_TICKS_PER_SECOND: u64 = 10;

// At 10 ticks/s, 4 ticks per slot implies that leader rotation and voting will happen
// every 400 ms. A fast voting cadence ensures faster finality and convergence
pub const DEFAULT_TICKS_PER_SLOT: u64 = 4;

// 1 Epoch = 400 * 8192 ms ~= 55 minutes
pub const DEFAULT_SLOTS_PER_EPOCH: u64 = 8192;

// Storage segment configuration
pub const DEFAULT_SLOTS_PER_SEGMENT: u64 = 1024;

// 4 times longer than the max_lockout to allow enough time for PoRep (128 slots)
pub const DEFAULT_SLOTS_PER_TURN: u64 = 32 * 4;

pub const NUM_CONSECUTIVE_LEADER_SLOTS: u64 = 4;

/// The time window of recent block hash values that the bank will track the signatures
/// of over. Once the bank discards a block hash, it will reject any transactions that use
/// that `recent_blockhash` in a transaction. Lowering this value reduces memory consumption,
/// but requires clients to update its `recent_blockhash` more frequently. Raising the value
/// lengthens the time a client must wait to be certain a missing transaction will
/// not be processed by the network.
pub const MAX_HASH_AGE_IN_SECONDS: usize = 120;

// This must be <= MAX_HASH_AGE_IN_SECONDS, otherwise there's risk for DuplicateSignature errors
pub const MAX_RECENT_BLOCKHASHES: usize = MAX_HASH_AGE_IN_SECONDS;

// The maximum age of a blockhash that will be accepted by the leader
pub const MAX_PROCESSING_AGE: usize = MAX_RECENT_BLOCKHASHES / 2;

/// This is maximum time consumed in forwarding a transaction from one node to next, before
/// it can be processed in the target node
#[cfg(feature = "cuda")]
pub const MAX_TRANSACTION_FORWARDING_DELAY: usize = 2;

/// More delay is expected if CUDA is not enabled (as signature verification takes longer)
#[cfg(not(feature = "cuda"))]
pub const MAX_TRANSACTION_FORWARDING_DELAY: usize = 6;

pub fn duration_as_ns(d: &Duration) -> u64 {
    d.as_secs() * 1_000_000_000 + u64::from(d.subsec_nanos())
}

pub fn duration_as_us(d: &Duration) -> u64 {
    (d.as_secs() * 1000 * 1000) + (u64::from(d.subsec_nanos()) / 1_000)
}

pub fn duration_as_ms(d: &Duration) -> u64 {
    (d.as_secs() * 1000) + (u64::from(d.subsec_nanos()) / 1_000_000)
}

pub fn duration_as_s(d: &Duration) -> f32 {
    d.as_secs() as f32 + (d.subsec_nanos() as f32 / 1_000_000_000.0)
}

pub fn timestamp() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("create timestamp in timing");
    duration_as_ms(&now)
}

/// Converts a slot to a storage segment. Does not indicate that a segment is complete.
pub fn get_segment_from_slot(rooted_slot: Slot, slots_per_segment: u64) -> Segment {
    ((rooted_slot + (slots_per_segment - 1)) / slots_per_segment)
}

/// Given a slot returns the latest complete segment, if no segment could possibly be complete
/// for a given slot it returns `None` (i.e if `slot < slots_per_segment`)
pub fn get_complete_segment_from_slot(
    rooted_slot: Slot,
    slots_per_segment: u64,
) -> Option<Segment> {
    let completed_segment = rooted_slot / slots_per_segment;
    if rooted_slot < slots_per_segment {
        None
    } else {
        Some(completed_segment)
    }
}

/// Slot is a unit of time given to a leader for encoding,
///  is some some number of Ticks long.  Use a u64 to count them.
pub type Slot = u64;

/// A segment is some number of slots stored by replicators
pub type Segment = u64;

/// Epoch is a unit of time a given leader schedule is honored,
///  some number of Slots.  Use a u64 to count them.
pub type Epoch = u64;

#[cfg(test)]
mod tests {
    use super::*;

    fn get_segments(slot: Slot, slots_per_segment: u64) -> (Segment, Segment) {
        (
            get_segment_from_slot(slot, slots_per_segment),
            get_complete_segment_from_slot(slot, slots_per_segment).unwrap(),
        )
    }

    #[test]
    fn test_complete_segment_impossible() {
        // slot < slots_per_segment so there can be no complete segments
        assert_eq!(get_complete_segment_from_slot(5, 10), None);
    }

    #[test]
    fn test_segment_conversion() {
        let (current, complete) = get_segments(2048, 1024);
        assert_eq!(current, complete);
        let (current, complete) = get_segments(2049, 1024);
        assert!(complete < current);
    }
}
