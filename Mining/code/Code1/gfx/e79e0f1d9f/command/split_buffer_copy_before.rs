    fn split_buffer_copy(
        copies: &mut Vec<Copy>, r: &com::BufferImageCopy, image: &n::Image
    ) {
        let buffer_width = if r.buffer_width == 0 {
            r.image_extent.width
        } else {
            r.buffer_width
        };
        let buffer_height = if r.buffer_height == 0 {
            r.image_extent.height
        } else {
            r.buffer_height
        };
        let row_pitch = div(buffer_width, image.block_dim.0 as _) * image.bytes_per_block as u32;
        let slice_pitch = div(buffer_height, image.block_dim.1 as _) * row_pitch;
        let is_pitch_aligned = row_pitch % d3d12::D3D12_TEXTURE_DATA_PITCH_ALIGNMENT == 0;

        for layer in r.image_layers.layers.clone() {
            let img_subresource = image
                .calc_subresource(r.image_layers.level as _, layer as _, 0);
            let layer_relative = (layer - r.image_layers.layers.start) as u32;
            let layer_offset = r.buffer_offset as u64 + (layer_relative * slice_pitch * r.image_extent.depth) as u64;
            let aligned_offset = layer_offset & !(d3d12::D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT as u64 - 1);
            if layer_offset == aligned_offset && is_pitch_aligned {
                // trivial case: everything is aligned, ready for copying
                copies.push(Copy {
                    footprint_offset: aligned_offset,
                    footprint: r.image_extent,
                    row_pitch,
                    img_subresource,
                    img_offset: r.image_offset,
                    buf_offset: image::Offset::ZERO,
                    copy_extent: r.image_extent,
                });
            } else if is_pitch_aligned {
                // buffer offset is not aligned
                assert_eq!(image.block_dim, (1, 1)); // TODO
                let row_pitch_texels = row_pitch / image.bytes_per_block as u32;
                let gap = (layer_offset - aligned_offset) as i32;
                let buf_offset = image::Offset {
                    x: gap % row_pitch as i32,
                    y: (gap % slice_pitch as i32) / row_pitch as i32,
                    z: gap / slice_pitch as i32,
                };
                let footprint = image::Extent {
                    width: buf_offset.x as u32 + r.image_extent.width,
                    height: buf_offset.y as u32 + r.image_extent.height,
                    depth: buf_offset.z as u32 + r.image_extent.depth,
                };
                if r.image_extent.width + buf_offset.x as u32 <= row_pitch_texels {
                    // we can map it to the aligned one and adjust the offsets accordingly
                    copies.push(Copy {
                        footprint_offset: aligned_offset,
                        footprint,
                        row_pitch,
                        img_subresource,
                        img_offset: r.image_offset,
                        buf_offset,
                        copy_extent: r.image_extent,
                    });
                } else {
                    // split the copy region into 2 that suffice the previous condition
                    assert!(buf_offset.x as u32 <= row_pitch_texels);
                    let half = row_pitch_texels - buf_offset.x as u32;
                    assert!(half <= r.image_extent.width);

                    copies.push(Copy {
                        footprint_offset: aligned_offset,
                        footprint: image::Extent {
                            width: row_pitch_texels,
                            .. footprint
                        },
                        row_pitch,
                        img_subresource,
                        img_offset: r.image_offset,
                        buf_offset,
                        copy_extent: image::Extent {
                            width: half,
                            .. r.image_extent
                        },
                    });
                    copies.push(Copy {
                        footprint_offset: aligned_offset,
                        footprint: image::Extent {
                            width: r.image_extent.width - half,
                            height: footprint.height + 1,
                            depth: footprint.depth,
                        },
                        row_pitch,
                        img_subresource,
                        img_offset: image::Offset {
                            x: r.image_offset.x + half as i32,
                            .. r.image_offset
                        },
                        buf_offset: image::Offset {
                            x: 0,
                            .. buf_offset
                        },
                        copy_extent: image::Extent {
                            width: r.image_extent.width - half,
                            .. r.image_extent
                        },
                    });
                }
            } else {
                // worst case: row by row copy
                assert_eq!(image.block_dim, (1, 1)); // TODO
                for z in 0 .. r.image_extent.depth {
                    for y in 0 .. r.image_extent.height {
                        // an image row starts non-aligned
                        let row_offset = layer_offset +
                            z as u64 * slice_pitch as u64 +
                            y as u64 * row_pitch as u64;
                        let aligned_offset = row_offset & !(d3d12::D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT as u64 - 1);
                        let next_aligned_offset = aligned_offset + d3d12::D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT as u64;
                        let cut_row_texels = (next_aligned_offset - row_offset) / image.bytes_per_block as u64;
                        let cut_width = cmp::min(r.image_extent.width, cut_row_texels as image::Size);
                        let gap_texels = (row_offset - aligned_offset) as image::Size / image.bytes_per_block as image::Size;
                        // this is a conservative row pitch that should be compatible with both copies
                        let max_unaligned_pitch = (r.image_extent.width + gap_texels) * image.bytes_per_block as u32;
                        let row_pitch = (max_unaligned_pitch | (d3d12::D3D12_TEXTURE_DATA_PITCH_ALIGNMENT - 1)) + 1;

                        copies.push(Copy {
                            footprint_offset: aligned_offset,
                            footprint: image::Extent {
                                width: cut_width + gap_texels,
                                height: 1,
                                depth: 1,
                            },
                            row_pitch,
                            img_subresource,
                            img_offset: image::Offset {
                                x: r.image_offset.x,
                                y: r.image_offset.y + y as i32,
                                z: r.image_offset.z + z as i32,
                            },
                            buf_offset: image::Offset {
                                x: gap_texels as i32,
                                y: 0,
                                z: 0,
                            },
                            copy_extent: image::Extent {
                                width: cut_width,
                                height: 1,
                                depth: 1,
                            },
                        });

                        // and if it crosses a pitch alignment - we copy the rest separately
                        if cut_width == r.image_extent.width {
                            continue;
                        }
                        let leftover = r.image_extent.width - cut_width;

                        copies.push(Copy {
                            footprint_offset: next_aligned_offset,
                            footprint: image::Extent {
                                width: leftover,
                                height: 1,
                                depth: 1,
                            },
                            row_pitch,
                            img_subresource,
                            img_offset: image::Offset {
                                x: r.image_offset.x + cut_width as i32,
                                y: r.image_offset.y + y as i32,
                                z: r.image_offset.z + z as i32,
                            },
                            buf_offset: image::Offset::ZERO,
                            copy_extent: image::Extent {
                                width: leftover,
                                height: 1,
                                depth: 1,
                            },
                        });
                    }
                }
            }
        }
    }
