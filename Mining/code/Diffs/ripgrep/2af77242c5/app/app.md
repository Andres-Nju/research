File_Code/ripgrep/2af77242c5/app/app_after.rs --- Rust
1198         "\                                                                                                                                              1198         "\
1199 Specify which regular expression engine to use. When you choose a regex engine,                                                                         1199 Specify which regular expression engine to use. When you choose a regex engine,
1200 it applies that choice for every regex provided to ripgrep (e.g., via multiple                                                                          1200 it applies that choice for every regex provided to ripgrep (e.g., via multiple
1201 -e/--regexp or -f/--file flags).                                                                                                                        1201 -e/--regexp or -f/--file flags).
1202                                                                                                                                                         1202 
1203 Accepted values are 'default', 'pcre2', or 'auto'.                                                                                                      1203 Accepted values are 'default', 'pcre2', or 'auto'.
1204                                                                                                                                                         1204 
1205 The default value is 'default', which is the fastest and should be good for                                                                             1205 The default value is 'default', which is the fastest and should be good for
1206 most use cases. The 'pcre2' engine is generally useful when you want to use                                                                             1206 most use cases. The 'pcre2' engine is generally useful when you want to use
1207 features such as look-around or backreferences. 'auto' will dynamically choose                                                                          1207 features such as look-around or backreferences. 'auto' will dynamically choose
1208 between supported regex engines depending on the features used in a pattern on                                                                          1208 between supported regex engines depending on the features used in a pattern on
1209 a best effort basis.                                                                                                                                    1209 a best effort basis.
1210                                                                                                                                                         1210 
1211 Note that the 'pcre2' engine is an optional ripgrep feature. If PCRE2 wasn't                                                                            1211 Note that the 'pcre2' engine is an optional ripgrep feature. If PCRE2 wasn't
1212 including in your build of ripgrep, then using this flag will result in ripgrep                                                                         1212 included in your build of ripgrep, then using this flag will result in ripgrep
1213 printing an error message and exiting.                                                                                                                  1213 printing an error message and exiting.
1214                                                                                                                                                         1214 
1215 This overrides previous uses of --pcre2 and --auto-hybrid-regex flags.                                                                                  1215 This overrides previous uses of --pcre2 and --auto-hybrid-regex flags.
1216 "                                                                                                                                                       1216 "

