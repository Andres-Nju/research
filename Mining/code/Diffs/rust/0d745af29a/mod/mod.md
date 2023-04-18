File_Code/rust/0d745af29a/mod/mod_after.rs --- 1/2 --- Rust
                                                                                                                                                            18 use ops;

File_Code/rust/0d745af29a/mod/mod_after.rs --- 2/2 --- Rust
2225         #[rustc_inherit_overflow_checks]                                                                                                                  .. 
2226         pub fn next_power_of_two(self) -> Self {                                                                                                        2226         pub fn next_power_of_two(self) -> Self {
....                                                                                                                                                         2227             // Call the trait to get overflow checks
2227             self.one_less_than_next_power_of_two() + 1                                                                                                  2228             ops::Add::add(self.one_less_than_next_power_of_two(), 1)

