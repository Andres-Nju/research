File_Code/rust/70ae43fee7/vec_deque/vec_deque_after.rs --- 1/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1027                 ring: unsafe { self.buffer_as_slice() },                                                                                                1027                 ring: unsafe { self.buffer_as_mut_slice() },

File_Code/rust/70ae43fee7/vec_deque/vec_deque_after.rs --- 2/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
2596                             let src = right_edge + right_offset;                                                                                        2596                             let src: isize = (right_edge + right_offset) as isize;
2597                             ptr::swap(buf.add(i), buf.add(src));                                                                                        2597                             ptr::swap(buf.add(i), buf.offset(src));

