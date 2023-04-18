File_Code/rust/1eff1d414f/diagnostics/diagnostics_after.rs --- Rust
1572 E0603: r##"                                                                                                                                             1572 E0603: r##"
1573 A private item was used outside its scope.                                                                                                              1573 A private item was used outside its scope.
1574                                                                                                                                                         1574 
1575 Erroneous code example:                                                                                                                                 1575 Erroneous code example:
1576                                                                                                                                                         1576 
1577 ```compile_fail,E0603                                                                                                                                   1577 ```compile_fail,E0603
1578 mod SomeModule {                                                                                                                                        1578 mod SomeModule {
1579     const PRIVATE: u32 = 0x_a_bad_1dea_u32; // This const is private, so we                                                                             1579     const PRIVATE: u32 = 0x_a_bad_1dea_u32; // This const is private, so we
1580                                             // can't use it outside of the                                                                              1580                                             // can't use it outside of the
1581                                             // `SomeModule` module.                                                                                     1581                                             // `SomeModule` module.
1582 }                                                                                                                                                       1582 }
1583                                                                                                                                                         1583 
1584 println!("const value: {}", SomeModule::PRIVATE); // error: constant `CONSTANT`                                                                         1584 println!("const value: {}", SomeModule::PRIVATE); // error: constant `PRIVATE`
1585                                                   //        is private                                                                                  1585                                                   //        is private
1586 ```                                                                                                                                                     1586 ```
1587                                                                                                                                                         1587 
1588 In order to fix this error, you need to make the item public by using the `pub`                                                                         1588 In order to fix this error, you need to make the item public by using the `pub`
1589 keyword. Example:                                                                                                                                       1589 keyword. Example:
1590                                                                                                                                                         1590 
1591 ```                                                                                                                                                     1591 ```
1592 mod SomeModule {                                                                                                                                        1592 mod SomeModule {
1593     pub const PRIVATE: u32 = 0x_a_bad_1dea_u32; // We set it public by using the                                                                        1593     pub const PRIVATE: u32 = 0x_a_bad_1dea_u32; // We set it public by using the
1594                                                 // `pub` keyword.                                                                                       1594                                                 // `pub` keyword.
1595 }                                                                                                                                                       1595 }
1596                                                                                                                                                         1596 
1597 println!("const value: {}", SomeModule::PRIVATE); // ok!                                                                                                1597 println!("const value: {}", SomeModule::PRIVATE); // ok!
1598 ```                                                                                                                                                     1598 ```
1599 "##,                                                                                                                                                    1599 "##,

