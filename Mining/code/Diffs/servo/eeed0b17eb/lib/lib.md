File_Code/servo/eeed0b17eb/lib/lib_after.rs --- 1/2 --- Rust
67 use euclid::TypedSize2D;                                                                                                                                    

File_Code/servo/eeed0b17eb/lib/lib_after.rs --- 2/2 --- Rust
394 impl<T: MallocSizeOf, U> MallocSizeOf for TypedSize2D<T, U> {                                                                                            393 impl<T: MallocSizeOf, U> MallocSizeOf for euclid::TypedSize2D<T, U> {
395     fn size_of(&self, ops: &mut MallocSizeOfOps) -> usize {                                                                                              394     fn size_of(&self, ops: &mut MallocSizeOfOps) -> usize {
396         let n = self.width.size_of(ops) + self.width.size_of(ops);                                                                                       395         self.width.size_of(ops) + self.height.size_of(ops)
397         assert!(n == 0);    // It would be very strange to have a non-zero value here...                                                                     
398         n                                                                                                                                                    

