Codes/tauri/d1cd6d102e/appimage_bundle/bundle_project_after.rs --- Rust
17     remove_dir_all(&package_dir).or_else(|e| Err(e.to_string()))?;                                                                                        17     remove_dir_all(&package_dir)
                                                                                                                                                             18       .chain_err(|| format!("Failed to remove old {}", package_base_name))?;

