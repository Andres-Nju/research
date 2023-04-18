File_Code/lapce/200083fcae/view/view_after.rs --- Rust
  .                                                                                                                                                          624                     // If autosave is enabled, and the content is a file that we can save,
624                     if data.config.editor.autosave_interval > 0 {                                                                                        625                     if data.config.editor.autosave_interval > 0
                                                                                                                                                             626                         && editor.content.is_file()

