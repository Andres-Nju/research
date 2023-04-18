File_Code/wezterm/4705fb96fb/unix/unix_after.rs --- Rust
395     #[cfg(target_os = "illumos")]                                                                                                                          . 
396     let domain = libc::AF_UNIX;                                                                                                                            . 
397     #[cfg(not(target_os = "illumos"))]                                                                                                                     . 
398     let domain = libc::PF_LOCAL;                                                                                                                           . 
399                                                                                                                                                            . 
400     let res = unsafe { libc::socketpair(domain, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };                                                               395     let res = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };

