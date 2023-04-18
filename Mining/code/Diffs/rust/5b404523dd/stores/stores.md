File_Code/rust/5b404523dd/stores/stores_after.rs --- 1/2 --- Rust
29 // CHECK: [[VAR:%[0-9]+]] = bitcast [4 x i8]* %y to i32*                                                                                                  29 // CHECK: store i32 %{{.*}}, i32* %{{.*}}, align 1
30 // CHECK: store i32 %{{.*}}, i32* [[VAR]], align 1                                                                                                        30 // CHECK: [[VAR:%[0-9]+]] = bitcast i32* %{{.*}} to [4 x i8]*

File_Code/rust/5b404523dd/stores/stores_after.rs --- 2/2 --- Rust
40 // CHECK: [[VAR:%[0-9]+]] = bitcast %Bytes* %y to i32*                                                                                                    40 // CHECK: store i32 %{{.*}}, i32* %{{.*}}, align 1
41 // CHECK: store i32 %{{.*}}, i32* [[VAR]], align 1                                                                                                        41 // CHECK: [[VAR:%[0-9]+]] = bitcast i32* %{{.*}} to %Bytes*

