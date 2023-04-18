File_Code/rust/e2cc1571ba/no-patterns-in-args/no-patterns-in-args_after.rs --- Rust
 .                                                                                                                                                           13                         //~^ NOTE pattern not allowed in foreign function
13                         //~^ NOTE this is a recent error                                                                                                  14                         //~| NOTE this is a recent error
14     fn f2(&arg: u8); //~ ERROR patterns aren't allowed in foreign function declarations                                                                   15     fn f2(&arg: u8); //~ ERROR patterns aren't allowed in foreign function declarations
..                                                                                                                                                           16                      //~^ NOTE pattern not allowed in foreign function
15     fn f3(arg @ _: u8); //~ ERROR patterns aren't allowed in foreign function declarations                                                                17     fn f3(arg @ _: u8); //~ ERROR patterns aren't allowed in foreign function declarations
..                                                                                                                                                           18                         //~^ NOTE pattern not allowed in foreign function
16                         //~^ NOTE this is a recent error                                                                                                  19                         //~| NOTE this is a recent error

