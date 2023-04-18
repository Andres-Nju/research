File_Code/gfx/7817caa487/command/command_after.rs --- Rust
1789                 if let Ok(drawable) = swapchain.borrow().take_drawable(index) {                                                                         1789                 let drawable = swapchain.borrow().take_drawable(index)?;
1790                     command_buffer.present_drawable(&drawable);                                                                                         1790                 command_buffer.present_drawable(&drawable);
1791                 }                                                                                                                                       .... 
1792             }                                                                                                                                           1791             }
1793             command_buffer.commit();                                                                                                                    1792             command_buffer.commit();
....                                                                                                                                                         1793             Ok(())
1794         });                                                                                                                                             1794         })?;

