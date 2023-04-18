pub fn get_complete_segment_from_slot(
    rooted_slot: Slot,
    slots_per_segment: u64,
) -> Option<Segment> {
    let current_segment = get_segment_from_slot(rooted_slot, slots_per_segment);
    if current_segment == 1 {
        None
    } else if rooted_slot < (current_segment * slots_per_segment) {
        Some(current_segment - 1)
    } else {
        Some(current_segment)
    }
}
