    fn free_sets(&mut self, descriptor_sets: &[DescriptorSet]) {
        for descriptor_set in descriptor_sets {
            for binding_info in &descriptor_set.binding_infos {
                if let Some(ref view_range) = binding_info.view_range {
                    if HeapProperties::from(view_range.ty).has_view {
                        self.heap_srv_cbv_uav.free_handles(view_range.handle);
                    }
                    
                }
                if let Some(ref sampler_range) = binding_info.sampler_range {
                    if HeapProperties::from(sampler_range.ty).has_sampler {
                        self.heap_sampler.free_handles(sampler_range.handle);
                    }
                }
            }
        }
    }
