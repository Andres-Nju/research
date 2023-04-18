File_Code/gfx/03e0a52e2e/device/device_after.rs --- 1/2 --- Rust
403                 self.invalidate_mapped_memory_ranges(Some((memory, 0..len)));                                                                            403                 self.invalidate_mapped_memory_ranges(Some((memory, range.clone())));

File_Code/gfx/03e0a52e2e/device/device_after.rs --- 2/2 --- Rust
443         let len = writer.range.end - writer.range.start;                                                                                                 ... 
444         self.flush_mapped_memory_ranges(Some((writer.memory, 0..len)));                                                                                  443         self.flush_mapped_memory_ranges(Some((writer.memory, writer.range.clone())));

