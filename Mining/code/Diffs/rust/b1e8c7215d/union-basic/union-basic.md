File_Code/rust/b1e8c7215d/union-basic/union-basic_after.rs --- 1/3 --- Rust
13 // FIXME: This test case makes little-endian assumptions.                                                                                                   
14 // ignore-s390x                                                                                                                                             
15 // ignore-sparc                                                                                                                                             

File_Code/rust/b1e8c7215d/union-basic/union-basic_after.rs --- 2/3 --- Rust
42         assert_eq!(w.b, 1);                                                                                                                               38         assert_eq!(w.b.to_le(), 1);

File_Code/rust/b1e8c7215d/union-basic/union-basic_after.rs --- 3/3 --- Rust
63         assert_eq!(w.b, 1);                                                                                                                               59         assert_eq!(w.b.to_le(), 1);

