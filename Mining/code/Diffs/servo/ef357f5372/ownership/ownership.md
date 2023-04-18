File_Code/servo/ef357f5372/ownership/ownership_after.rs --- 1/2 --- Rust
207     pub fn into_box<U>(self) -> Box<T> where U: HasBoxFFI<FFIType = T> {                                                                                 207     pub fn into_box<U>(self) -> Box<U> where U: HasBoxFFI<FFIType = T> {

File_Code/servo/ef357f5372/ownership/ownership_after.rs --- 2/2 --- Rust
241     pub fn into_box_opt<U>(self) -> Option<Box<T>> where U: HasBoxFFI<FFIType = T> {                                                                     241     pub fn into_box_opt<U>(self) -> Option<Box<U>> where U: HasBoxFFI<FFIType = T> {

