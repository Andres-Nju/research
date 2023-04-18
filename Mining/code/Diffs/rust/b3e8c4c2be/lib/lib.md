File_Code/rust/b3e8c4c2be/lib/lib_after.rs --- 1/2 --- Rust
                                                                                                                                                           169         fn GetLastError() -> DWORD;

File_Code/rust/b3e8c4c2be/lib/lib_after.rs --- 2/2 --- Rust
233             debug_assert!(err != 0);                                                                                                                     234             debug_assert!(err != 0, "Failed to free heap memory: {}", GetLastError());
234         } else {                                                                                                                                         235         } else {
235             let header = get_header(ptr);                                                                                                                236             let header = get_header(ptr);
236             let err = HeapFree(GetProcessHeap(), 0, header.0 as LPVOID);                                                                                 237             let err = HeapFree(GetProcessHeap(), 0, header.0 as LPVOID);
237             debug_assert!(err != 0);                                                                                                                     238             debug_assert!(err != 0, "Failed to free heap memory: {}", GetLastError());

