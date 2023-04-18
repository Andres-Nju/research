File_Code/servo/16252633bd/properties/properties_after.rs --- Text (1394 errors, exceeded DFT_PARSE_ERROR_LIMIT)
2181         if context.layout_parent_style.writing_mode != style.writing_mode &&                                                                            2181         let our_writing_mode = style.get_inheritedbox().clone_writing_mode();
                                                                                                                                                             2182         let parent_writing_mode = context.layout_parent_style.get_inheritedbox().clone_writing_mode();
                                                                                                                                                             2183         if our_writing_mode != parent_writing_mode &&

