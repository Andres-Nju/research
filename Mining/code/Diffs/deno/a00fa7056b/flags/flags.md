File_Code/deno/a00fa7056b/flags/flags_after.rs --- Rust
 99           "                                                                                                                                               99           "
100 Fetch and compile remote dependencies recursively.                                                                                                       100 Fetch and compile remote dependencies recursively.
101                                                                                                                                                          101 
102 Downloads all statically imported scripts and save them in local                                                                                         102 Downloads all statically imported scripts and save them in local
103 cache, without running the code. No future import network requests                                                                                       103 cache, without running the code. No future import network requests
104 would be made unless --reload is specified.                                                                                                              104 would be made unless --reload is specified.
105                                                                                                                                                          105 
106   # Downloads all dependencies                                                                                                                           106   # Downloads all dependencies
107   deno fetch https://deno.land/std/http/file_server.ts                                                                                                   107   deno fetch https://deno.land/std/http/file_server.ts
108   # Once cached, static imports no longer send network requests                                                                                          108   # Once cached, static imports no longer send network requests
109   deno https://deno.land/std/http/file_server.ts                                                                                                         109   deno run -A https://deno.land/std/http/file_server.ts
110 ",                                                                                                                                                       110 ",

