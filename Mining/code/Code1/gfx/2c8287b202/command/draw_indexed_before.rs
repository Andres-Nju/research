    fn draw_indexed(
        &mut self,
        indices: Range<IndexCount>,
        base_vertex: VertexOffset,
        instances: Range<InstanceCount>,
    ) {
        let (ref buffer, ref offset, ref index_type) = *self.inner_ref().index_buffer.as_ref().expect("must bind index buffer");
        let primitive_type = self.inner_ref().primitive_type;
        let encoder = self.expect_renderpass();
        let index_offset = match *index_type {
            MTLIndexType::UInt16 => indices.start as u64 * 2,
            MTLIndexType::UInt32 => indices.start as u64 * 4,
        };

        unsafe {
            msg_send![encoder,
                drawIndexedPrimitives: primitive_type
                indexCount: (indices.end - indices.start) as NSUInteger
                indexType: index_type
                indexBuffer: buffer
                indexBufferOffset: (index_offset + offset) as NSUInteger
                instanceCount: (instances.end - instances.start) as NSUInteger
                baseVertex: base_vertex as NSUInteger
                baseInstance: instances.start as NSUInteger
            ];
        }
    }
