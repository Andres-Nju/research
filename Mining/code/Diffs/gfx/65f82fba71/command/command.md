File_Code/gfx/65f82fba71/command/command_after.rs --- Rust
  .                                                                                                                                                          647             let layer_relative = (layer - r.image_layers.layers.start) as u32;
647             let layer_offset = r.buffer_offset as u64 + (layer as u32 * slice_pitch * r.image_extent.depth) as u64;                                      648             let layer_offset = r.buffer_offset as u64 + (layer_relative * slice_pitch * r.image_extent.depth) as u64;

