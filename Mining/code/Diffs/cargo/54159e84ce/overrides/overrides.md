File_Code/cargo/54159e84ce/overrides/overrides_after.rs --- Rust
818             "\                                                                                                                                           818             "\
819 [UPDATING] [..]                                                                                                                                          819 [UPDATING] [..]
820 warning: path override for crate `a` has altered the original list of                                                                                    820 warning: path override for crate `a` has altered the original list of
821 dependencies; the dependency on `bar` was either added or                                                                                                821 dependencies; the dependency on `bar` was either added or
822 modified to not match the previously resolved version                                                                                                    822 modified to not match the previously resolved version
823                                                                                                                                                          823 
824 This is currently allowed but is known to produce buggy behavior with spurious                                                                           824 This is currently allowed but is known to produce buggy behavior with spurious
825 recompiles and changes to the crate graph. Path overrides unfortunately were                                                                             825 recompiles and changes to the crate graph. Path overrides unfortunately were
826 never intended to support this feature, so for now this message is just a                                                                                826 never intended to support this feature, so for now this message is just a
827 warning. In the future, however, this message will become a hard error.                                                                                  827 warning. In the future, however, this message will become a hard error.
828                                                                                                                                                          828 
829 To change the dependency graph via an override it's recommended to use the                                                                               829 To change the dependency graph via an override it's recommended to use the
830 `[replace]` feature of Cargo instead of the path override feature. This is                                                                               830 `[replace]` feature of Cargo instead of the path override feature. This is
831 documented online at the url below for more information.                                                                                                 831 documented online at the url below for more information.
832                                                                                                                                                          832 
833 http://doc.crates.io/specifying-dependencies.html#overriding-dependencies                                                                                833 https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#overriding-dependencies
834                                                                                                                                                          834 
835 [DOWNLOADING] [..]                                                                                                                                       835 [DOWNLOADING] [..]
836 [COMPILING] [..]                                                                                                                                         836 [COMPILING] [..]
837 [COMPILING] [..]                                                                                                                                         837 [COMPILING] [..]
838 [COMPILING] [..]                                                                                                                                         838 [COMPILING] [..]
839 [FINISHED] [..]                                                                                                                                          839 [FINISHED] [..]
840 ",                                                                                                                                                       840 ",

