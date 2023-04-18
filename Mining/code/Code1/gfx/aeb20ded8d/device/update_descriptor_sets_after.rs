    fn update_descriptor_sets(&self, writes: &[pso::DescriptorSetWrite<B>]) {
        // Create temporary non-shader visible views for uniform and storage buffers.
        let mut num_views = 0;
        for sw in writes {
            match sw.write {
                pso::DescriptorWrite::UniformBuffer(ref views) |
                pso::DescriptorWrite::StorageBuffer(ref views) => {
                    num_views += views.len();
                },
                _ => (),
            }
        }

        let mut raw = self.raw.clone();
        let mut handles = Vec::with_capacity(num_views);
        let _buffer_heap = if num_views != 0 {
            let mut heap = n::DescriptorCpuPool {
                heap: Self::create_descriptor_heap_impl(
                    &mut raw,
                    d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                    false,
                    num_views,
                ),
                offset: 0,
                size: 0,
                max_size: num_views as _,
            };
            // Create views
            for sw in writes {
                match sw.write {
                    pso::DescriptorWrite::UniformBuffer(ref views) => {
                        handles.extend(views.iter().map(|&(buffer, ref range)| {
                            let handle = heap.alloc_handles(1).cpu;
                            // Making the size field of buffer requirements for uniform
                            // buffers a multiple of 256 and setting the required offset
                            // alignment to 256 allows us to patch the size here.
                            // We can always enforce the size to be aligned to 256 for
                            // CBVs without going out-of-bounds.
                            let size = ((range.end - range.start) + 255) & !255;
                            let desc = d3d12::D3D12_CONSTANT_BUFFER_VIEW_DESC {
                                BufferLocation: unsafe { (*buffer.resource).GetGPUVirtualAddress() },
                                SizeInBytes: size as _,
                            };
                            unsafe { raw.CreateConstantBufferView(&desc, handle); }
                            handle
                        }));
                    }
                    pso::DescriptorWrite::StorageBuffer(ref views) => {
                        // StorageBuffer gets translated into a byte address buffer.
                        // We don't have to stride information to make it structured.
                        handles.extend(views.iter().map(|&(buffer, ref range)| {
                            let handle = heap.alloc_handles(1).cpu;
                            let mut desc = d3d12::D3D12_UNORDERED_ACCESS_VIEW_DESC {
                                Format: dxgiformat::DXGI_FORMAT_R32_TYPELESS,
                                ViewDimension: d3d12::D3D12_UAV_DIMENSION_BUFFER,
                                u: unsafe { mem::zeroed() },
                            };

                           *unsafe { desc.u.Buffer_mut() } = d3d12::D3D12_BUFFER_UAV {
                                FirstElement: range.start as _,
                                NumElements: ((range.end - range.start) / 4) as _,
                                StructureByteStride: 0,
                                CounterOffsetInBytes: 0,
                                Flags: d3d12::D3D12_BUFFER_UAV_FLAG_RAW,
                            };

                            unsafe {
                                raw.CreateUnorderedAccessView(buffer.resource, ptr::null_mut(), &desc, handle);
                            }
                            handle
                        }));
                    }
                    _ => {}
                }
            }

            Some(heap)
        } else {
            None
        };

        let mut cur_view = 0;
        self.update_descriptor_sets_impl(writes,
            d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            |dw, starts| match *dw {
                pso::DescriptorWrite::SampledImage(ref images) => {
                    starts.extend(images.iter().map(|&(ref srv, _layout)| srv.handle_srv.unwrap()));
                }
                pso::DescriptorWrite::UniformBuffer(ref views) |
                pso::DescriptorWrite::StorageBuffer(ref views) => {
                    starts.extend(&handles[cur_view .. cur_view + views.len()]);
                    cur_view += views.len();
                }
                pso::DescriptorWrite::StorageImage(ref images) => {
                    starts.extend(images.iter().map(|&(ref uav, _layout)| uav.handle_uav.unwrap()));
                }
                pso::DescriptorWrite::Sampler(_) => (), // done separately
                _ => unimplemented!()
            });

        self.update_descriptor_sets_impl(writes,
            d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
            |dw, starts| match *dw {
                pso::DescriptorWrite::Sampler(ref samplers) => {
                    starts.extend(samplers.iter().map(|sm| sm.handle))
                }
                _ => ()
            });
    }
