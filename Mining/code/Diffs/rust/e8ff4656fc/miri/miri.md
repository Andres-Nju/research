File_Code/rust/e8ff4656fc/miri/miri_after.rs --- 1/2 --- Rust
35 extern "C" {                                                                                                                                                
36     #[link_name = "\x01??_7type_info@@6B@"]                                                                                                                 
37     static TYPE_INFO_VTABLE: *const u8;                                                                                                                     
38 }                                                                                                                                                           

File_Code/rust/e8ff4656fc/miri/miri_after.rs --- 2/2 --- Rust
44     pVFTable: unsafe { &TYPE_INFO_VTABLE } as *const _ as *const _,                                                                                       39     pVFTable: core::ptr::null(),

