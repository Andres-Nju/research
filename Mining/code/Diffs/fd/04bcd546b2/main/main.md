File_Code/fd/04bcd546b2/main/main_after.rs --- 1/2 --- Rust
178             if !cfg!(any(target_os = "macos", target_os = "dragonfly", target_os = "freebsd")) {                                                         178             if !cfg!(any(target_os = "macos", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")) {

File_Code/fd/04bcd546b2/main/main_after.rs --- 2/2 --- Rust
203                     if colored_output {                                                                                                                  203                     if !cfg!(any(target_os = "netbsd", target_os = "openbsd")) && colored_output {
                                                                                                                                                             204                         // -G is not available in NetBSD's and OpenBSD's ls

