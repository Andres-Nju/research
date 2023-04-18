File_Code/deno/86684799c4/flags/flags_after.rs --- Rust
186           "Run a program given a filename or url to the source code.                                                                                     186           "Run a program given a filename or url to the source code.
187                                                                                                                                                          187 
188 By default all programs are run in sandbox without access to disk, network or                                                                            188 By default all programs are run in sandbox without access to disk, network or
189 ability to spawn subprocesses.                                                                                                                           189 ability to spawn subprocesses.
190                                                                                                                                                          190 
191   deno run https://deno.land/welcome.ts                                                                                                                  191   deno run https://deno.land/welcome.ts
192                                                                                                                                                          192 
193   # run program with permission to read from disk and listen to network                                                                                  193   # run program with permission to read from disk and listen to network
194   deno run --allow-net --allow-read https://deno.land/std/http/file_server.ts                                                                            194   deno run --allow-net --allow-read https://deno.land/std/http/file_server.ts
195                                                                                                                                                          195 
196   # run program with permission to read whitelist files from disk and listen to nework                                                                   196   # run program with permission to read whitelist files from disk and listen to network
197   deno run --allow-net --allow-read=$(pwd) https://deno.land/std/http/file_server.ts                                                                     197   deno run --allow-net --allow-read=$(pwd) https://deno.land/std/http/file_server.ts 
198                                                                                                                                                          198 
199   # run program with all permissions                                                                                                                     199   # run program with all permissions
200   deno run -A https://deno.land/std/http/file_server.ts",                                                                                                200   deno run -A https://deno.land/std/http/file_server.ts",

