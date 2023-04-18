File_Code/rust/f7961126b5/rwlock/rwlock_after.rs --- Rust
48         // We also check whether there this lock is already write locked. This                                                                            48         // We also check whether this lock is already write locked. This
49         // is only possible if it was write locked by the current thread and                                                                              49         // is only possible if it was write locked by the current thread and
50         // the implementation allows recursive locking. The POSIX standard                                                                                50         // the implementation allows recursive locking. The POSIX standard
51         // doesn't require recursivly locking a rwlock to deadlock, but we can't                                                                          51         // doesn't require recursively locking a rwlock to deadlock, but we can't

