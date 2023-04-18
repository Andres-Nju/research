    fn destroy_compute_pipeline(&self, pipeline: B::ComputePipeline);

    ///
    fn create_framebuffer<I>(
        &self,
        pass: &B::RenderPass,
        attachments: I,
        extent: image::Extent,
    ) -> Result<B::Framebuffer, FramebufferError>
    where
        I: IntoIterator,
        I::Item: Borrow<B::ImageView>;

    /// Destroys a framebuffer.
    ///
    /// The framebuffer shouldn't be destroy before any submitted command buffer,
    /// which references the framebuffer, has finished execution.
    fn destroy_framebuffer(&self, buf: B::Framebuffer);

    ///
    fn create_shader_module(
        &self, spirv_data: &[u8]
    ) -> Result<B::ShaderModule, ShaderError>;

    ///
    fn destroy_shader_module(&self, shader: B::ShaderModule);

    /// Create a new buffer (unbound).
    ///
    /// The created buffer won't have associated memory until `bind_buffer_memory` is called.
    fn create_buffer(
        &self, size: u64, usage: buffer::Usage,
    ) -> Result<B::UnboundBuffer, buffer::CreationError>;

    ///
    fn get_buffer_requirements(&self, buf: &B::UnboundBuffer) -> Requirements;

    /// Bind memory to a buffer.
    ///
    /// The unbound buffer will be consumed because the binding is *immutable*.
    /// Be sure to check that there is enough memory available for the buffer.
    /// Use `get_buffer_requirements` to acquire the memory requirements.
    fn bind_buffer_memory(
        &self, memory: &B::Memory, offset: u64, buf: B::UnboundBuffer
    ) -> Result<B::Buffer, BindError>;

    /// Destroys a buffer.
    ///
    /// The buffer shouldn't be destroyed before any submitted command buffer,
    /// which references the images, has finished execution.
    fn destroy_buffer(&self, B::Buffer);

    ///
    fn create_buffer_view<R: RangeArg<u64>>(
        &self, buf: &B::Buffer, fmt: Option<format::Format>, range: R
    ) -> Result<B::BufferView, buffer::ViewCreationError>;

    ///
    fn destroy_buffer_view(&self, view: B::BufferView);

    ///
    fn create_image(
        &self, kind: image::Kind, mip_levels: image::Level, format: format::Format,
        tiling: image::Tiling, usage: image::Usage, storage_flags: image::StorageFlags,
    ) -> Result<B::UnboundImage, image::CreationError>;

    ///
    fn get_image_requirements(&self, image: &B::UnboundImage) -> Requirements;

    ///
    fn get_image_subresource_footprint(
        &self, image: &B::Image, subresource: image::Subresource
    ) -> image::SubresourceFootprint;

    ///
    fn bind_image_memory(
        &self, &B::Memory, offset: u64, B::UnboundImage
    ) -> Result<B::Image, BindError>;

    /// Destroys an image.
    ///
    /// The image shouldn't be destroyed before any submitted command buffer,
    /// which references the images, has finished execution.
    fn destroy_image(&self, image: B::Image);

    ///
    fn create_image_view(
        &self,
        image: &B::Image,
        view_kind: image::ViewKind,
        format: format::Format,
        swizzle: format::Swizzle,
        range: image::SubresourceRange,
    ) -> Result<B::ImageView, image::ViewError>;

    ///
    fn destroy_image_view(&self, view: B::ImageView);

    ///
    fn create_sampler(&self, info: image::SamplerInfo) -> B::Sampler;

    ///
    fn destroy_sampler(&self, sampler: B::Sampler);

    /// Create a descriptor pool.
    ///
    /// Descriptor pools allow allocation of descriptor sets.
    /// Ihe pool can't be modified directly, only through updating descriptor sets.
    fn create_descriptor_pool<I>(&self, max_sets: usize, descriptor_ranges: I) -> B::DescriptorPool
    where
        I: IntoIterator,
        I::Item: Borrow<pso::DescriptorRangeDesc>;

    ///
    fn destroy_descriptor_pool(&self, pool: B::DescriptorPool);

    /// Create a descriptor set layout.
    fn create_descriptor_set_layout<I, J>(
        &self, bindings: I, immutable_samplers: J
    ) -> B::DescriptorSetLayout
    where
        I: IntoIterator,
        I::Item: Borrow<pso::DescriptorSetLayoutBinding>,
        J: IntoIterator,
        J::Item: Borrow<B::Sampler>;

    ///
    fn destroy_descriptor_set_layout(&self, layout: B::DescriptorSetLayout);

    ///
    fn write_descriptor_sets<'a, I, J>(&self, write_iter: I)
    where
        I: IntoIterator<Item = pso::DescriptorSetWrite<'a, B, J>>,
        J: IntoIterator,
        J::Item: Borrow<pso::Descriptor<'a, B>>;

    ///
    fn copy_descriptor_sets<'a, I>(&self, copy_iter: I)
    where
        I: IntoIterator,
        I::Item: Borrow<pso::DescriptorSetCopy<'a, B>>;

    ///
    fn map_memory<R>(&self, memory: &B::Memory, range: R) -> Result<*mut u8, mapping::Error>
    where
        R: RangeArg<u64>;

    ///
    fn flush_mapped_memory_ranges<'a, I, R>(&self, ranges: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(&'a B::Memory, R)>,
        R: RangeArg<u64>;

    ///
    fn invalidate_mapped_memory_ranges<'a, I, R>(&self, ranges: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(&'a B::Memory, R)>,
        R: RangeArg<u64>;

    ///
    fn unmap_memory(&self, memory: &B::Memory);

    /// Acquire a mapping Reader.
    ///
    /// The accessible slice will correspond to the specified range (in bytes).
    fn acquire_mapping_reader<'a, T>(&self, memory: &'a B::Memory, range: Range<u64>)
        -> Result<mapping::Reader<'a, B, T>, mapping::Error>
    where
        T: Copy,
    {
        let len = range.end - range.start;
        let count = len as usize / mem::size_of::<T>();
        self.map_memory(memory, range.clone())
            .map(|ptr| unsafe {
                let start_ptr = ptr as *const _;
                self.invalidate_mapped_memory_ranges(Some((memory, range.clone())));

                mapping::Reader {
                    slice: slice::from_raw_parts(start_ptr, count),
                    memory,
                    released: false,
                }
            })
    }
