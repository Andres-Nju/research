File_Code/rust/c1ed84e6ec/error/error_after.rs --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
131     /// Sets the message passed in via `message`, then adds the span labels for you, before applying                                                     131     /// Sets the message passed in via `message` and adds span labels before handing control back
132     /// further modifications in `emit`. It's up to you to call emit(), stash(..), etc. within the                                                       132     /// to `emit` to do any final processing. It's the caller's responsibility to call emit(),
133     /// `emit` method. If you don't need to do any additional processing, just use                                                                       133     /// stash(), etc. within the `emit` function to dispose of the diagnostic properly.
134     /// struct_generic.                                                                                                                                      

