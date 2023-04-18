     fn copy_buffer_to_image<T>(
         &mut self,
        src: &n::Buffer,
        dst: &n::Image,
        _: image::ImageLayout,
        regions: T,
     ) where
         T: IntoIterator,
         T::Item: Borrow<command::BufferImageCopy>,
     {
        let old_offset = self.buf.offset;

        for region in regions {
            let r = region.borrow().clone();
            let cmd = match dst.kind {
                n::ImageKind::Surface(s) => Command::CopyBufferToSurface(src.raw, s, r),
                n::ImageKind::Texture(t) => Command::CopyBufferToTexture(src.raw, t, r),
            };
            self.push_cmd(cmd);
        }

        if self.buf.offset == old_offset {
            error!("At least one region must be specified");
        }
    }
