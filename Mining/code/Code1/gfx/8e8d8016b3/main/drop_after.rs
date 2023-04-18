    fn drop(&mut self) {
        self.device.wait_idle().unwrap();
        unsafe {
            // TODO: When ManuallyDrop::take (soon to be renamed to ManuallyDrop::read) is stabilized we should use that instead.
            self.device
                .destroy_descriptor_pool(ManuallyDrop::into_inner(ptr::read(&self.desc_pool)));
            self.device
                .destroy_descriptor_set_layout(ManuallyDrop::into_inner(ptr::read(
                    &self.set_layout,
                )));

            self.device
                .destroy_buffer(ManuallyDrop::into_inner(ptr::read(&self.vertex_buffer)));
            self.device
                .destroy_buffer(ManuallyDrop::into_inner(ptr::read(
                    &self.image_upload_buffer,
                )));
            self.device
                .destroy_image(ManuallyDrop::into_inner(ptr::read(&self.image_logo)));
            self.device
                .destroy_image_view(ManuallyDrop::into_inner(ptr::read(&self.image_srv)));
            self.device
                .destroy_sampler(ManuallyDrop::into_inner(ptr::read(&self.sampler)));
            for p in self.cmd_pools.drain(..) {
                self.device.destroy_command_pool(p);
            }
            for s in self.submission_complete_semaphores.drain(..) {
                self.device.destroy_semaphore(s);
            }
            for f in self.submission_complete_fences.drain(..) {
                self.device.destroy_fence(f);
            }
            self.device
                .destroy_render_pass(ManuallyDrop::into_inner(ptr::read(&self.render_pass)));
            self.surface
              .unconfigure_swapchain(&self.device);
            self.device
                .free_memory(ManuallyDrop::into_inner(ptr::read(&self.buffer_memory)));
            self.device
                .free_memory(ManuallyDrop::into_inner(ptr::read(&self.image_memory)));
            self.device.free_memory(ManuallyDrop::into_inner(ptr::read(
                &self.image_upload_memory,
            )));
            self.device
                .destroy_graphics_pipeline(ManuallyDrop::into_inner(ptr::read(&self.pipeline)));
            self.device
                .destroy_pipeline_layout(ManuallyDrop::into_inner(ptr::read(
                    &self.pipeline_layout,
                )));
        }
        println!("DROPPED!");
    }
