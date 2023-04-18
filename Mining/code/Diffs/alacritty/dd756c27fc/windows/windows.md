File_Code/alacritty/dd756c27fc/windows/windows_after.rs --- 1/2 --- Rust
44     fn load_selection(&self) -> Result<String, Self::Err> {                                                                                                 
45         let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();                                                                                  
46         ctx.get_contents().map_err(Error::Clipboard)                                                                                                        
47     }                                                                                                                                                       

File_Code/alacritty/dd756c27fc/windows/windows_after.rs --- 2/2 --- Rust
..                                                                                                                                                           61         // No such thing on Windows
66         self.0.set_contents(contents.into()).map_err(Error::Clipboard)                                                                                    62         Ok(())

