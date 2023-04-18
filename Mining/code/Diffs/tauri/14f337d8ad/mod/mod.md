File_Code/tauri/14f337d8ad/mod/mod_after.rs --- 1/2 --- Rust
29 //!     "pubkey": ""                                                                                                                                      29 //!     "pubkey": "YOUR_UPDATER_PUBLIC_KEY_HERE"
30 //! }                                                                                                                                                     30 //! }
31 //! ```                                                                                                                                                   31 //! ```
32 //!                                                                                                                                                       32 //!
33 //! The required keys are "active" and "endpoints", others are optional.                                                                                  33 //! The required keys are "active", "endpoints" and "pubkey"; others are optional.

File_Code/tauri/14f337d8ad/mod/mod_after.rs --- 2/2 --- Rust
41 //! "pubkey" if present must be a valid public-key generated with Tauri cli. See [Signing updates](#signing-updates).                                     41 //! "pubkey" must be a valid public-key generated with Tauri cli. See [Signing updates](#signing-updates).

