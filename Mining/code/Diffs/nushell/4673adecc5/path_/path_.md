File_Code/nushell/4673adecc5/path_/path__after.rs --- Rust
25         r#"There are three ways to represent a path:                                                                                                      25         r#"There are three ways to represent a path:
26                                                                                                                                                           26 
27 * As a path literal, e.g., '/home/viking/spam.txt'                                                                                                        27 * As a path literal, e.g., '/home/viking/spam.txt'
28 * As a structured path: a table with 'parent', 'stem', and 'extension' (and                                                                               28 * As a structured path: a table with 'parent', 'stem', and 'extension' (and
29 * 'prefix' on Windows) columns. This format is produced by the 'path parse'                                                                               29 * 'prefix' on Windows) columns. This format is produced by the 'path parse'
30   subcommand.                                                                                                                                             30   subcommand.
31 * As an inner list of path parts, e.g., '[[ / home viking spam.txt ]]'.                                                                                   31 * As a list of path parts, e.g., '[ / home viking spam.txt ]'. Splitting into
32   Splitting into parts is done by the `path split` command.                                                                                               32   parts is done by the `path split` command.
33                                                                                                                                                           33 
34 All subcommands accept all three variants as an input. Furthermore, the 'path                                                                             34 All subcommands accept all three variants as an input. Furthermore, the 'path
35 join' subcommand can be used to join the structured path or path parts back into                                                                          35 join' subcommand can be used to join the structured path or path parts back into
36 the path literal."#                                                                                                                                       36 the path literal."#

