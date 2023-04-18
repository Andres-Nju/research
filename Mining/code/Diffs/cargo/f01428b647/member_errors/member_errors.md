File_Code/cargo/f01428b647/member_errors/member_errors_after.rs --- 1/2 --- Rust
2 use cargo::core::{compiler::CompileMode, Workspace};                                                                                                       2 use cargo::core::{compiler::CompileMode, Shell, Workspace};
3 use cargo::ops::{self, CompileOptions};                                                                                                                    3 use cargo::ops::{self, CompileOptions};
4 use cargo::util::{config::Config, errors::ManifestError};                                                                                                  4 use cargo::util::{config::Config, errors::ManifestError};
5                                                                                                                                                            5 
.                                                                                                                                                            6 use crate::support::install::cargo_home;
6 use crate::support::project;                                                                                                                               7 use crate::support::project;
                                                                                                                                                             8 use crate::support::registry;

File_Code/cargo/f01428b647/member_errors/member_errors_after.rs --- 2/2 --- Rust
  .                                                                                                                                                          144     // Prevent this test from accessing the network by setting up .cargo/config.
  .                                                                                                                                                          145     registry::init();
142     let config = Config::default().unwrap();                                                                                                             146     let config = Config::new(Shell::new(), cargo_home(), cargo_home());

