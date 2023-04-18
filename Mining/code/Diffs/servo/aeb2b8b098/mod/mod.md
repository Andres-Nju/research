File_Code/servo/aeb2b8b098/mod/mod_after.rs --- Rust
97     pub unsafe fn new<'a>(atom: *mut nsIAtom) -> &'a mut Self {                                                                                           97     pub unsafe fn new<'a>(atom: *const nsIAtom) -> &'a mut Self {

