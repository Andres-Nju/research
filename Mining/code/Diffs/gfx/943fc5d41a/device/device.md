File_Code/gfx/943fc5d41a/device/device_after.rs --- Rust
522                     primitive_restart_enable: vk::FALSE,                                                                                                 522                     primitive_restart_enable: match desc.input_assembler.primitive_restart {
                                                                                                                                                             523                         pso::PrimitiveRestart::U16|pso::PrimitiveRestart::U32 => vk::TRUE,
                                                                                                                                                             524                         pso::PrimitiveRestart::Disabled => vk::FALSE
                                                                                                                                                             525                     }

