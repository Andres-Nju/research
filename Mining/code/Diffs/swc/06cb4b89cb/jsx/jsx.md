File_Code/swc/06cb4b89cb/jsx/jsx_after.rs --- Rust
82             JSXAttrOrSpread::SpreadElement(ref n) => emit!(n),                                                                                            82             JSXAttrOrSpread::SpreadElement(ref n) => {
                                                                                                                                                             83                 punct!("{");
                                                                                                                                                             84                 emit!(n);
                                                                                                                                                             85                 punct!("}");
                                                                                                                                                             86             }

