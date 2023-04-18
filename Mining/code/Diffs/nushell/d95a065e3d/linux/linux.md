File_Code/nushell/d95a065e3d/linux/linux_after.rs --- Rust
92             if with_thread {                                                                                                                              92             if with_thread {
93                 if let Ok(iter) = proc.tasks() {                                                                                                          93                 if let Ok(iter) = proc.tasks() {
94                     collect_task(iter, &mut base_tasks);                                                                                                  94                     collect_task(iter, &mut base_tasks);
95                 }                                                                                                                                         95                 }
..                                                                                                                                                           96             }
96                 base_procs.push((proc.pid(), proc, io, time));                                                                                            97             base_procs.push((proc.pid(), proc, io, time));
97             }                                                                                                                                                

