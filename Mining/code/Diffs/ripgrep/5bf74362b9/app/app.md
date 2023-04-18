File_Code/ripgrep/5bf74362b9/app/app_after.rs --- Rust
1390         "\                                                                                                                                              1390         "\
1391 Include or exclude files and directories for searching that match the given                                                                             1391 Include or exclude files and directories for searching that match the given
1392 glob. This always overrides any other ignore logic. Multiple glob flags may be                                                                          1392 glob. This always overrides any other ignore logic. Multiple glob flags may be
1393 used. Globbing rules match .gitignore globs. Precede a glob with a ! to exclude                                                                         1393 used. Globbing rules match .gitignore globs. Precede a glob with a ! to exclude
1394 it. If multiple globs match a file or directory, the glob given later in the                                                                            1394 it. If multiple globs match a file or directory, the glob given later in the
1395 command line takes precedence.                                                                                                                          1395 command line takes precedence.
1396                                                                                                                                                         1396 
1397 As an extension, globs support specifying alternatives: *-g ab{c,d}* is                                                                                 1397 As an extension, globs support specifying alternatives: *-g ab{c,d}* is
1398 equivalet to *-g abc -g abd*. Empty alternatives like *-g ab{,c}* are not                                                                               1398 equivalent to *-g abc -g abd*. Empty alternatives like *-g ab{,c}* are not
1399 currently supported. Note that this syntax extension is also currently enabled                                                                          1399 currently supported. Note that this syntax extension is also currently enabled
1400 in gitignore files, even though this syntax isn't supported by git itself.                                                                              1400 in gitignore files, even though this syntax isn't supported by git itself.
1401 ripgrep may disable this syntax extension in gitignore files, but it will                                                                               1401 ripgrep may disable this syntax extension in gitignore files, but it will
1402 always remain available via the -g/--glob flag.                                                                                                         1402 always remain available via the -g/--glob flag.
1403                                                                                                                                                         1403 
1404 When this flag is set, every file and directory is applied to it to test for                                                                            1404 When this flag is set, every file and directory is applied to it to test for
1405 a match. So for example, if you only want to search in a particular directory                                                                           1405 a match. So for example, if you only want to search in a particular directory
1406 'foo', then *-g foo* is incorrect because 'foo/bar' does not match the glob                                                                             1406 'foo', then *-g foo* is incorrect because 'foo/bar' does not match the glob
1407 'foo'. Instead, you should use *-g 'foo/**'*.                                                                                                           1407 'foo'. Instead, you should use *-g 'foo/**'*.
1408 "                                                                                                                                                       1408 "

