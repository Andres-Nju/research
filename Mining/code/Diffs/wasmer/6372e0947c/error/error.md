File_Code/wasmer/6372e0947c/error/error_after.rs --- 1/2 --- Rust
11     static LAST_ERROR: RefCell<Option<Box<Error>>> = RefCell::new(None);                                                                                  11     static LAST_ERROR: RefCell<Option<Box<dyn Error>>> = RefCell::new(None);

File_Code/wasmer/6372e0947c/error/error_after.rs --- 2/2 --- Rust
21 pub(crate) fn take_last_error() -> Option<Box<Error>> {                                                                                                   21 pub(crate) fn take_last_error() -> Option<Box<dyn Error>> {

