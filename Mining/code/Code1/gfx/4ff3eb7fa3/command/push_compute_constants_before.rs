    unsafe fn push_compute_constants(
        &mut self,
        layout: &n::PipelineLayout,
        offset: u32,
        constants: &[u32],
    ) {
        unsafe {
            self.device.0.cmd_push_constants(
                self.raw,
                layout.raw,
                vk::ShaderStageFlags::COMPUTE,
                offset * 4,
                memory::cast_slice(constants),
            );
        }
    }
