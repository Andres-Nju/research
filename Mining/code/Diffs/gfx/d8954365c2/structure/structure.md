File_Code/gfx/d8954365c2/structure/structure_after.rs --- Rust
44                 use std::mem::size_of;                                                                                                                    44                 use std::mem::{size_of, transmute};
45                 use $crate::pso::buffer::{Element, ElemOffset};                                                                                           45                 use $crate::pso::buffer::{Element, ElemOffset};
..                                                                                                                                                           46                 // using "1" here as a simple non-zero pointer addres
46                 let tmp: &$root = unsafe{ ::std::mem::uninitialized() };                                                                                  47                 let tmp: &$root = unsafe{ transmute(1usize) };

