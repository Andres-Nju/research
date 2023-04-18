File_Code/rust/18391b62f3/sync/sync_after.rs --- Rust
495     /// Tries to initialize the inner value by calling the closure while ensuring that no-one else                                                       495     /// Initializes the inner value if it wasn't already done by calling the provided closure. It
496     /// can access the value in the mean time by holding a lock for the duration of the closure.                                                         496     /// ensures that no-one else can access the value in the mean time by holding a lock for the
497     /// If the value was already initialized the closure is not called and `false` is returned,                                                          497     /// duration of the closure.
498     /// otherwise if the value from the closure initializes the inner value, `true` is returned                                                          498     /// A reference to the inner value is returned.

