    fn copy_image_to_buffer<T>(
        &mut self,
        src: &n::Image,
        _: image::ImageLayout,
        dst: &n::Buffer,
        regions: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<command::BufferImageCopy>,
    {
        let old_size = self.buf.size;

        for region in regions {
            let r = region.borrow().clone();
            let cmd = match src.kind {
                n::ImageKind::Surface(s) => Command::CopySurfaceToBuffer(s, dst.raw, r),
                n::ImageKind::Texture(t) => Command::CopyTextureToBuffer(t, dst.raw, r),
            };
            self.push_cmd(cmd);
        }

        if self.buf.size == old_size {
            error!("At least one region must be specified");
        }
    }
