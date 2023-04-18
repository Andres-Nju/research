File_Code/nushell/97cedeb324/lib/lib_after.rs --- 1/2 --- Rust
34             r#"                                                                                                                                           34             r#"
35                 open los_tres_amigos.txt                                                                                                                  35                 open los_tres_amigos.txt
36                 | from-csv                                                                                                                                36                 | from-csv
37                 | get rusty_luck                                                                                                                          37                 | get rusty_luck
38                 | str --to-int                                                                                                                            38                 | str to-int
39                 | math sum                                                                                                                                39                 | math sum
40                 | echo "$it"                                                                                                                              40                 | echo "$it"
41             "#,                                                                                                                                           41             "#,

File_Code/nushell/97cedeb324/lib/lib_after.rs --- 2/2 --- Rust
46             r#"open los_tres_amigos.txt | from-csv | get rusty_luck | str --to-int | math sum | echo "$it""#                                              46             r#"open los_tres_amigos.txt | from-csv | get rusty_luck | str to-int | math sum | echo "$it""#

