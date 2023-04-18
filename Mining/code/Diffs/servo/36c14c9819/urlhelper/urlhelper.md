File_Code/servo/36c14c9819/urlhelper/urlhelper_after.rs --- Rust
14     pub fn SameOrigin(url_a: &ServoUrl, url_b: &ServoUrl) -> bool {                                                                                         
15         url_a.origin() == url_b.origin()                                                                                                                    
16     }                                                                                                                                                       

