File_Code/ripgrep/79271fcb33/search_buffer/search_buffer_after.rs --- Rust
1 /*!                                                                                                                                                        1 /*!
2 The `search_buffer` module is responsible for searching a single file all in a                                                                             2 The `search_buffer` module is responsible for searching a single file all in a
3 single buffer. Typically, the source of the buffer is a memory map. This can                                                                               3 single buffer. Typically, the source of the buffer is a memory map. This can
4 be useful for when memory maps are faster than streaming search.                                                                                           4 be useful for when memory maps are faster than streaming search.
5                                                                                                                                                            5 
6 Note that this module doesn't quite support everything that `search_stream`                                                                                6 Note that this module doesn't quite support everything that `search_stream`
7 Notdoes. ably, showing contexts.                                                                                                                           7 does. Notably, showing contexts.
8 */                                                                                                                                                         8 */

