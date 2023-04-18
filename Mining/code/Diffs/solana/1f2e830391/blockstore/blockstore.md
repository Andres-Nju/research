File_Code/solana/1f2e830391/blockstore/blockstore_after.rs --- Rust
   .                                                                                                                                                         4281     let current = nofile.rlim_cur;
4281     if nofile.rlim_cur < desired_nofile {                                                                                                               4282     if current < desired_nofile {
4282         nofile.rlim_cur = desired_nofile;                                                                                                               4283         nofile.rlim_cur = desired_nofile;
4283         if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &nofile) } != 0 {                                                                              4284         if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &nofile) } != 0 {
4284             error!(                                                                                                                                     4285             error!(
4285                 "Unable to increase the maximum open file descriptor limit to {}",                                                                      4286                 "Unable to increase the maximum open file descriptor limit to {} from {}",
4286                 desired_nofile                                                                                                                          4287                 nofile.rlim_cur, current,

