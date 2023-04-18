File_Code/gfx/9be5301c40/raw/raw_after.rs --- 1/4 --- Rust
211     fn bind_index_buffer(&mut self, buffer::IndexBufferView<B>);                                                                                         211     fn bind_index_buffer(&mut self, view: buffer::IndexBufferView<B>);

File_Code/gfx/9be5301c40/raw/raw_after.rs --- 2/4 --- Rust
279     fn set_blend_constants(&mut self, pso::ColorValue);                                                                                                  279     fn set_blend_constants(&mut self, color: pso::ColorValue);

File_Code/gfx/9be5301c40/raw/raw_after.rs --- 3/4 --- Rust
323     fn bind_graphics_pipeline(&mut self, &B::GraphicsPipeline);                                                                                          323     fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline);

File_Code/gfx/9be5301c40/raw/raw_after.rs --- 4/4 --- Rust
348     fn bind_compute_pipeline(&mut self, &B::ComputePipeline);                                                                                            348     fn bind_compute_pipeline(&mut self, pipeline: &B::ComputePipeline);

