File_Code/cargo/ffd5e0fcce/layout/layout_after.rs --- 1/2 --- Rust
81     /// Calcuate the paths for build output, lock the build directory, and return as a Layout.                                                            81     /// Calculate the paths for build output, lock the build directory, and return as a Layout.
82     ///                                                                                                                                                   82     ///
83     /// This function will block if the directory is already locked.                                                                                      83     /// This function will block if the directory is already locked.
84     ///                                                                                                                                                   84     ///
85     /// Differs from `at` in that it calculates the root path from the workspace target directory,                                                        85     /// Differs from `at` in that this calculates the root path from the workspace target directory,

File_Code/cargo/ffd5e0fcce/layout/layout_after.rs --- 2/2 --- Rust
101     /// Calcuate the paths for build output, lock the build directory, and return as a Layout.                                                           101     /// Calculate the paths for build output, lock the build directory, and return as a Layout.

