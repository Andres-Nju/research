    fn update_buffer(
        &mut self,
        dst: &native::Buffer,
        offset: buffer::Offset,
        data: &[u8],
    ) {
        let inner = self.inner();
        let src = inner.device.new_buffer_with_data(
            data.as_ptr() as _,
            data.len() as _,
            metal::MTLResourceOptions::StorageModePrivate,
        );
        inner.retained_buffers.push(src.clone());

        let command = soft::BlitCommand::CopyBuffer {
            src,
            dst: dst.raw.clone(),
            region: com::BufferCopy {
                src: 0,
                dst: offset,
                size: data.len() as _,
            },
        };
        inner.sink.blit_commands(iter::once(command));
    }
