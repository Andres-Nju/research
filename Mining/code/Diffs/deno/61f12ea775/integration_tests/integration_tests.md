File_Code/deno/61f12ea775/integration_tests/integration_tests_after.rs --- Rust
   .                                                                                                                                                         1892   // Port 4600 is chosen to not colide with those used by tools/http_server.py
1892   let (_, err, code) = util::run_and_collect_output(                                                                                                    1893   let (_, err, code) = util::run_and_collect_output(
1893                         "run --allow-net=localhost complex_permissions_test.ts netListen localhost:4545 localhost:4546 localhost:4547",                 1894                         "run --allow-net=localhost complex_permissions_test.ts netListen localhost:4600",

