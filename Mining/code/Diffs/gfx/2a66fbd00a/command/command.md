File_Code/gfx/2a66fbd00a/command/command_after.rs --- Rust
745                         let max_unaligned_pitch = r.image_extent.width * image.bytes_per_block as u32;                                                   745                         let max_unaligned_pitch = (r.image_extent.width + gap_texels) * image.bytes_per_block as u32;
746                         let row_pitch = (max_unaligned_pitch | d3d12::D3D12_TEXTURE_DATA_PITCH_ALIGNMENT) + 1;                                           746                         let row_pitch = (max_unaligned_pitch | (d3d12::D3D12_TEXTURE_DATA_PITCH_ALIGNMENT - 1)) + 1;

