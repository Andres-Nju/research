File_Code/cargo/a8a944c43d/job_queue/job_queue_after.rs --- 1/2 --- Rust
                                                                                                                                                           185                             if self.active > 0 {
                                                                                                                                                           186                                 error = Some(human("build failed"));

File_Code/cargo/a8a944c43d/job_queue/job_queue_after.rs --- 2/2 --- Rust
187                             if self.active > 0 {                                                                                                         ... 
188                                 cx.config.shell().say(                                                                                                   188                                 cx.config.shell().say(
189                                             "Build failed, waiting for other \                                                                           189                                             "Build failed, waiting for other \
190                                              jobs to finish...", YELLOW)?;                                                                               190                                              jobs to finish...", YELLOW)?;
191                             }                                                                                                                            191                             }
192                             if error.is_none() {                                                                                                         192                             if error.is_none() {
193                                 error = Some(human("build failed"));                                                                                     193                                 error = Some(e);

