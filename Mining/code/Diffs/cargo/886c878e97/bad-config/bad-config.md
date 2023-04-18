File_Code/cargo/886c878e97/bad-config/bad-config_after.rs --- Rust
114                 execs().with_status(101).with_stderr("\                                                                                                  114                 execs().with_status(101).with_stderr("\
115 [ERROR] Couldn't load Cargo configuration                                                                                                                115 [ERROR] Failed to create project `foo` at `[..]`
116                                                                                                                                                          116 
117 Caused by:                                                                                                                                               117 Caused by:
118   failed to merge key `foo` between files:                                                                                                               118   Couldn't load Cargo configuration
119   file 1: [..]foo[..]foo[..]config                                                                                                                       119 
120   file 2: [..]foo[..]config                                                                                                                              120 Caused by:
121                                                                                                                                                          121   failed to merge key `foo` between files:
122 Caused by:                                                                                                                                               122   file 1: [..]foo[..]foo[..]config
123   expected integer, but found string                                                                                                                     123   file 2: [..]foo[..]config
...                                                                                                                                                          124 
...                                                                                                                                                          125 Caused by:
...                                                                                                                                                          126   expected integer, but found string
124 "));                                                                                                                                                     127 "));

