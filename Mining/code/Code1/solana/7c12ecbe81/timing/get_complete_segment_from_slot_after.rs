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
