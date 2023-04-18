    fn destroy_compute_pipeline(&self, pipeline: B::ComputePipeline);

    /// Create a new framebuffer object
    fn create_framebuffer<I>(
        &self,
        pass: &B::RenderPass,
        attachments: I,
        extent: image::Extent,
    ) -> Result<B::Framebuffer, FramebufferError>
    where
        I: IntoIterator,
        I::Item: Borrow<B::ImageView>;

    /// Destroy a framebuffer.
    ///
    /// The framebuffer shouldn't be destroy before any submitted command buffer,
    /// which references the framebuffer, has finished execution.
    fn destroy_framebuffer(&self, buf: B::Framebuffer);

    /// Create a new shader module object through the SPIR-V binary data.
    ///
    /// Once a shader module has been created, any entry points it contains can be used in pipeline
    /// shader stages as described in *Compute Pipelines* and *Graphics Pipelines*.
    fn create_shader_module(&self, spirv_data: &[u8]) -> Result<B::ShaderModule, ShaderError>;

    /// Destroy a shader module module
    ///
    /// A shader module can be destroyed while pipelines created using its shaders are still in use.
    fn destroy_shader_module(&self, shader: B::ShaderModule);

    /// Create a new buffer (unbound).
    ///
    /// The created buffer won't have associated memory until `bind_buffer_memory` is called.
    fn create_buffer(
        &self,
        size: u64,
        usage: buffer::Usage,
    ) -> Result<B::UnboundBuffer, buffer::CreationError>;

    /// Get memory requirements for the unbound buffer
    fn get_buffer_requirements(&self, buf: &B::UnboundBuffer) -> Requirements;

    /// Bind memory to a buffer.
    ///
    /// The unbound buffer will be consumed because the binding is *immutable*.
    /// Be sure to check that there is enough memory available for the buffer.
    /// Use `get_buffer_requirements` to acquire the memory requirements.
    fn bind_buffer_memory(
        &self,
        memory: &B::Memory,
        offset: u64,
        buf: B::UnboundBuffer,
    ) -> Result<B::Buffer, BindError>;

    /// Destroy a buffer.
    ///
    /// The buffer shouldn't be destroyed before any submitted command buffer,
    /// which references the images, has finished execution.
    fn destroy_buffer(&self, B::Buffer);

    /// Create a new buffer view object
    fn create_buffer_view<R: RangeArg<u64>>(
        &self,
        buf: &B::Buffer,
        fmt: Option<format::Format>,
        range: R,
    ) -> Result<B::BufferView, buffer::ViewCreationError>;

    /// Destroy a buffer view object
    fn destroy_buffer_view(&self, view: B::BufferView);

    /// Create a new image object
    fn create_image(
        &self,
        kind: image::Kind,
        mip_levels: image::Level,
        format: format::Format,
        tiling: image::Tiling,
        usage: image::Usage,
        view_caps: image::ViewCapabilities,
    ) -> Result<B::UnboundImage, image::CreationError>;

    /// Get memory requirements for the unbound Image
    fn get_image_requirements(&self, image: &B::UnboundImage) -> Requirements;

    ///
    fn get_image_subresource_footprint(
        &self,
        image: &B::Image,
        subresource: image::Subresource,
    ) -> image::SubresourceFootprint;

    /// Bind device memory to an image object
    fn bind_image_memory(
        &self,
        &B::Memory,
        offset: u64,
        B::UnboundImage,
    ) -> Result<B::Image, BindError>;

    /// Destroy an image.
    ///
    /// The image shouldn't be destroyed before any submitted command buffer,
    /// which references the images, has finished execution.
    fn destroy_image(&self, image: B::Image);

    /// Create an image view from an existing image
    fn create_image_view(
        &self,
        image: &B::Image,
        view_kind: image::ViewKind,
        format: format::Format,
        swizzle: format::Swizzle,
        range: image::SubresourceRange,
    ) -> Result<B::ImageView, image::ViewError>;

    /// Destroy an image view object
    fn destroy_image_view(&self, view: B::ImageView);

    /// Create a new sampler object
    fn create_sampler(&self, info: image::SamplerInfo) -> B::Sampler;

    /// Destroy a sampler object
    fn destroy_sampler(&self, sampler: B::Sampler);

    /// Create a descriptor pool.
    ///
    /// Descriptor pools allow allocation of descriptor sets.
    /// The pool can't be modified directly, only through updating descriptor sets.
    fn create_descriptor_pool<I>(&self, max_sets: usize, descriptor_ranges: I) -> B::DescriptorPool
    where
        I: IntoIterator,
        I::Item: Borrow<pso::DescriptorRangeDesc>;

    /// Destroy a descriptor pool object
    ///
    /// When a pool is destroyed, all descriptor sets allocated from the pool are implicitly freed
    /// and become invalid. Descriptor sets allocated from a given pool do not need to be freed
    /// before destroying that descriptor pool.
    fn destroy_descriptor_pool(&self, pool: B::DescriptorPool);

    /// Create a descriptor set layout.
    ///
    /// A descriptor set layout object is defined by an array of zero or more descriptor bindings.
    /// Each individual descriptor binding is specified by a descriptor type, a count (array size)
    /// of the number of descriptors in the binding, a set of shader stages that **can** access the
    /// binding, and (if using immutable samplers) an array of sampler descriptors.
    fn create_descriptor_set_layout<I, J>(
        &self,
        bindings: I,
        immutable_samplers: J,
    ) -> B::DescriptorSetLayout
    where
        I: IntoIterator,
        I::Item: Borrow<pso::DescriptorSetLayoutBinding>,
        J: IntoIterator,
        J::Item: Borrow<B::Sampler>;

    /// Destroy a descriptor set layout object
    fn destroy_descriptor_set_layout(&self, layout: B::DescriptorSetLayout);

    /// Specifying the parameters of a descriptor set write operation
    fn write_descriptor_sets<'a, I, J>(&self, write_iter: I)
    where
        I: IntoIterator<Item = pso::DescriptorSetWrite<'a, B, J>>,
        J: IntoIterator,
        J::Item: Borrow<pso::Descriptor<'a, B>>;

    /// Structure specifying a copy descriptor set operation
    fn copy_descriptor_sets<'a, I>(&self, copy_iter: I)
    where
        I: IntoIterator,
        I::Item: Borrow<pso::DescriptorSetCopy<'a, B>>;

    /// Map a memory object into application address space
    ///
    /// Call `map_memory()` to retrieve a host virtual address pointer to a region of a mappable memory object
    fn map_memory<R>(&self, memory: &B::Memory, range: R) -> Result<*mut u8, mapping::Error>
    where
        R: RangeArg<u64>;

    /// Flush mapped memory ranges
    fn flush_mapped_memory_ranges<'a, I, R>(&self, ranges: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(&'a B::Memory, R)>,
        R: RangeArg<u64>;

    /// Invalidate ranges of non-coherent memory from the host caches
    fn invalidate_mapped_memory_ranges<'a, I, R>(&self, ranges: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(&'a B::Memory, R)>,
        R: RangeArg<u64>;

    /// Unmap a memory object once host access to it is no longer needed by the application
    fn unmap_memory(&self, memory: &B::Memory);

    /// Acquire a mapping Reader.
    ///
    /// The accessible slice will correspond to the specified range (in bytes).
    fn acquire_mapping_reader<'a, T>(
        &self,
        memory: &'a B::Memory,
        range: Range<u64>,
    ) -> Result<mapping::Reader<'a, B, T>, mapping::Error>
    where
        T: Copy,
    {
        let len = range.end - range.start;
        let count = len as usize / mem::size_of::<T>();
        self.map_memory(memory, range.clone()).map(|ptr| unsafe {
            let start_ptr = ptr as *const _;
            self.invalidate_mapped_memory_ranges(iter::once((memory, range.clone())));

            mapping::Reader {
                slice: slice::from_raw_parts(start_ptr, count),
                memory,
                released: false,
            }
        })
    }
