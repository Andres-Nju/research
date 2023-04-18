File_Code/servo/8ced5db704/glue/glue_after.rs --- Rust
   .                                                                                                                                                         1507                       let mut index = unsafe { (*keyframe).mPropertyValues.len() };
1507                       for (index, property) in animation.properties_changed.iter().enumerate() {                                                        1508                       for property in animation.properties_changed.iter() {
1508                           if !seen.has_transition_property_bit(&property) {                                                                             1509                           if !seen.has_transition_property_bit(&property) {
1509                               add_computed_property_value(keyframe, index, style, property);                                                            1510                               add_computed_property_value(keyframe, index, style, property);
                                                                                                                                                             1511                               index += 1;

