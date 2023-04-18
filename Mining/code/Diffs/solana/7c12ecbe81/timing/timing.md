File_Code/solana/7c12ecbe81/timing/timing_after.rs --- Rust
81     let current_segment = get_segment_from_slot(rooted_slot, slots_per_segment);                                                                          81     let completed_segment = rooted_slot / slots_per_segment;
82     if current_segment == 1 {                                                                                                                             82     if rooted_slot < slots_per_segment {
83         None                                                                                                                                              83         None
84     } else if rooted_slot < (current_segment * slots_per_segment) {                                                                                       84     } else {
85         Some(current_segment - 1)                                                                                                                         85         Some(completed_segment)
86     } else {                                                                                                                                              86     }
87         Some(current_segment)                                                                                                                                
88     }                                                                                                                                                        

