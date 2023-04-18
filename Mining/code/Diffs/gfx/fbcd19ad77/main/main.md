File_Code/gfx/fbcd19ad77/main/main_after.rs --- 1/2 --- Rust
                                                                                                                                                           490     for _ in 0..frame_images.len() {
                                                                                                                                                           491         image_acquire_semaphores.push(
                                                                                                                                                           492             device
                                                                                                                                                           493                 .create_semaphore()
                                                                                                                                                           494                 .expect("Could not create semaphore"),
                                                                                                                                                           495         );
                                                                                                                                                           496     }

File_Code/gfx/fbcd19ad77/main/main_after.rs --- 2/2 --- Rust
491         image_acquire_semaphores.push(                                                                                                                       
492             device                                                                                                                                           
493                 .create_semaphore()                                                                                                                          
494                 .expect("Could not create semaphore"),                                                                                                       
495         );                                                                                                                                                   

