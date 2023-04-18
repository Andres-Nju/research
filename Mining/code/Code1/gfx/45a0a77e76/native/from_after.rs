    fn from(ty: pso::DescriptorType) -> HeapProperties {
        match ty {
            pso::DescriptorType::Sampler => HeapProperties::new(false, true, false),
            pso::DescriptorType::CombinedImageSampler => HeapProperties::new(true, true, false),
            pso::DescriptorType::InputAttachment |
            pso::DescriptorType::SampledImage |
            pso::DescriptorType::UniformTexelBuffer |
            pso::DescriptorType::UniformBuffer => HeapProperties::new(true, false, false),
            pso::DescriptorType::StorageImage |
            pso::DescriptorType::StorageTexelBuffer |
            pso::DescriptorType::StorageBuffer => HeapProperties::new(true, false, true),
            pso::DescriptorType::UniformBufferDynamic |
            pso::DescriptorType::UniformImageDynamic => unimplemented!(),
        }

    }
