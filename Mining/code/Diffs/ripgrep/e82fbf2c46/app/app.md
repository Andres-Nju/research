File_Code/ripgrep/e82fbf2c46/app/app_after.rs --- 1/2 --- Rust
2063     const LONG: &str = long!("\                                                                                                                         2063     const LONG: &str = long!("\
2064 This flag enables sorting of results in ascending order. The possible values                                                                            2064 This flag enables sorting of results in ascending order. The possible values
2065 for this flag are:                                                                                                                                      2065 for this flag are:
2066                                                                                                                                                         2066 
2067     path        Sort by file path.                                                                                                                      2067     path        Sort by file path.
2068     modified    Sort by the last modified time on a file.                                                                                               2068     modified    Sort by the last modified time on a file.
2069     accessed    Sort by the last accessed time on a file.                                                                                               2069     accessed    Sort by the last accessed time on a file.
2070     created     Sort by the cretion time on a file.                                                                                                     2070     created     Sort by the creation time on a file.
2071     none        Do not sort results.                                                                                                                    2071     none        Do not sort results.
2072                                                                                                                                                         2072 
2073 If the sorting criteria isn't available on your system (for example, creation                                                                           2073 If the sorting criteria isn't available on your system (for example, creation
2074 time is not available on ext4 file systems), then ripgrep will attempt to                                                                               2074 time is not available on ext4 file systems), then ripgrep will attempt to
2075 detect this and print an error without searching any results. Otherwise, the                                                                            2075 detect this and print an error without searching any results. Otherwise, the
2076 sort order is unspecified.                                                                                                                              2076 sort order is unspecified.
2077                                                                                                                                                         2077 
2078 To sort results in reverse or descending order, use the --sortr flag. Also,                                                                             2078 To sort results in reverse or descending order, use the --sortr flag. Also,
2079 this flag overrides --sortr.                                                                                                                            2079 this flag overrides --sortr.
2080                                                                                                                                                         2080 
2081 Note that sorting results currently always forces ripgrep to abandon                                                                                    2081 Note that sorting results currently always forces ripgrep to abandon
2082 parallelism and run in a single thread.                                                                                                                 2082 parallelism and run in a single thread.
2083 ");                                                                                                                                                     2083 ");

File_Code/ripgrep/e82fbf2c46/app/app_after.rs --- 2/2 --- Rust
2096     const LONG: &str = long!("\                                                                                                                         2096     const LONG: &str = long!("\
2097 This flag enables sorting of results in descending order. The possible values                                                                           2097 This flag enables sorting of results in descending order. The possible values
2098 for this flag are:                                                                                                                                      2098 for this flag are:
2099                                                                                                                                                         2099 
2100     path        Sort by file path.                                                                                                                      2100     path        Sort by file path.
2101     modified    Sort by the last modified time on a file.                                                                                               2101     modified    Sort by the last modified time on a file.
2102     accessed    Sort by the last accessed time on a file.                                                                                               2102     accessed    Sort by the last accessed time on a file.
2103     created     Sort by the cretion time on a file.                                                                                                     2103     created     Sort by the creation time on a file.
2104     none        Do not sort results.                                                                                                                    2104     none        Do not sort results.
2105                                                                                                                                                         2105 
2106 If the sorting criteria isn't available on your system (for example, creation                                                                           2106 If the sorting criteria isn't available on your system (for example, creation
2107 time is not available on ext4 file systems), then ripgrep will attempt to                                                                               2107 time is not available on ext4 file systems), then ripgrep will attempt to
2108 detect this and print an error without searching any results. Otherwise, the                                                                            2108 detect this and print an error without searching any results. Otherwise, the
2109 sort order is unspecified.                                                                                                                              2109 sort order is unspecified.
2110                                                                                                                                                         2110 
2111 To sort results in ascending order, use the --sort flag. Also, this flag                                                                                2111 To sort results in ascending order, use the --sort flag. Also, this flag
2112 overrides --sort.                                                                                                                                       2112 overrides --sort.
2113                                                                                                                                                         2113 
2114 Note that sorting results currently always forces ripgrep to abandon                                                                                    2114 Note that sorting results currently always forces ripgrep to abandon
2115 parallelism and run in a single thread.                                                                                                                 2115 parallelism and run in a single thread.
2116 ");                                                                                                                                                     2116 ");

