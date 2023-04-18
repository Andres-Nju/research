    unsafe fn push_graphics_constants(
        &mut self,
        layout: &n::PipelineLayout,
        stages: pso::ShaderStageFlags,
        offset: u32,
        constants: &[u32],
    ) {
        unsafe {
            self.device.0.cmd_push_constants(
                self.raw,
                layout.raw,
                conv::map_stage_flags(stages),
                offset * 4,
                memory::cast_slice(constants),
            );
        }
    }
