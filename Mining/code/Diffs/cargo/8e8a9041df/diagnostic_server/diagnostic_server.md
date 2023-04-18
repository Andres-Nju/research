File_Code/cargo/8e8a9041df/diagnostic_server/diagnostic_server_after.rs --- 1/2 --- Rust
168                     "\                                                                                                                                   168                     "\
169 cannot prepare for the {} edition when it is enabled, so cargo cannot                                                                                    169 cannot prepare for the {} edition when it is enabled, so cargo cannot
170 automatically fix errors in `{}`                                                                                                                         170 automatically fix errors in `{}`
171                                                                                                                                                          171 
172 To prepare for the {0} edition you should first remove `edition = '{0}'` from                                                                            172 To prepare for the {0} edition you should first remove `edition = '{0}'` from
173 your `Cargo.toml` and then rerun this command. Once all warnings have been fixed                                                                         173 your `Cargo.toml` and then rerun this command. Once all warnings have been fixed
174 then you can re-enable the `edition` key in `Cargo.toml`. For some more                                                                                  174 then you can re-enable the `edition` key in `Cargo.toml`. For some more
175 information about transitioning to the {0} edition see:                                                                                                  175 information about transitioning to the {0} edition see:
176                                                                                                                                                          176 
177   https://rust-lang-nursery.github.io/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html                                     177   https://doc.rust-lang.org/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html
178 ",                                                                                                                                                       178 ",

File_Code/cargo/8e8a9041df/diagnostic_server/diagnostic_server_after.rs --- 2/2 --- Rust
195                     "\                                                                                                                                   195                     "\
196 cannot migrate to the idioms of the {} edition for `{}`                                                                                                  196 cannot migrate to the idioms of the {} edition for `{}`
197 because it is compiled {}, which doesn't match {0}                                                                                                       197 because it is compiled {}, which doesn't match {0}
198                                                                                                                                                          198 
199 consider migrating to the {0} edition by adding `edition = '{0}'` to                                                                                     199 consider migrating to the {0} edition by adding `edition = '{0}'` to
200 `Cargo.toml` and then rerunning this command; a more detailed transition                                                                                 200 `Cargo.toml` and then rerunning this command; a more detailed transition
201 guide can be found at                                                                                                                                    201 guide can be found at
202                                                                                                                                                          202 
203   https://rust-lang-nursery.github.io/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html                                     203   https://doc.rust-lang.org/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html
204 ",                                                                                                                                                       204 ",

