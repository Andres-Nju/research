File_Code/servo/3b6afb31c6/window/window_after.rs --- Text (10 errors, exceeded DFT_PARSE_ERROR_LIMIT)
730             (SHIFT, Key::Backspace) => {                                                                                                                   
731                 self.event_queue.borrow_mut().push(WindowEvent::Navigation(WindowNavigateMsg::Forward));                                                   
732             }                                                                                                                                              
733             (NONE, Key::Backspace) => {                                                                                                                    
734                 self.event_queue.borrow_mut().push(WindowEvent::Navigation(WindowNavigateMsg::Back));                                                      
735             }                                                                                                                                              

