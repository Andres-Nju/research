File_Code/solana/db6d291127/syscalls/syscalls_after.rs --- Rust
157     if invoke_context.is_feature_active(&disable_fees_sysvar::id()) {                                                                                    157     if !invoke_context.is_feature_active(&disable_fees_sysvar::id()) {

