File_Code/nushell/86faf753bd/into_sqlite/into_sqlite_after.rs --- Rust
120                 .map(|(name, sql_type)| format!("{name} {sql_type}"))                                                                                    120                 .map(|(name, sql_type)| format!("\"{name}\" {sql_type}"))

