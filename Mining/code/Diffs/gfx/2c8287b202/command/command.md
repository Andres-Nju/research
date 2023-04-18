File_Code/gfx/2c8287b202/command/command_after.rs --- Rust
1072         let (ref buffer, ref offset, ref index_type) = *self.inner_ref().index_buffer.as_ref().expect("must bind index buffer");                        1072         let (buffer, offset, index_type) = self.inner_ref().index_buffer.as_ref().cloned().expect("must bind index buffer");
1073         let primitive_type = self.inner_ref().primitive_type;                                                                                           1073         let primitive_type = self.inner_ref().primitive_type;
1074         let encoder = self.expect_renderpass();                                                                                                         1074         let encoder = self.expect_renderpass();
1075         let index_offset = match *index_type {                                                                                                          1075         let index_offset = match index_type {

