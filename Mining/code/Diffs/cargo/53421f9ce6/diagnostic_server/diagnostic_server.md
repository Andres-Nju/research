File_Code/cargo/53421f9ce6/diagnostic_server/diagnostic_server_after.rs --- 1/2 --- Rust
144                     "\                                                                                                                                   144                     "\
145 cannot prepare for the {} edition when it is enabled, so cargo cannot                                                                                    145 cannot prepare for the {} edition when it is enabled, so cargo cannot
146 automatically fix errors in `{}`                                                                                                                         146 automatically fix errors in `{}`
147                                                                                                                                                          147 
148 To prepare for the {0} edition you should first remove `edition = '{0}'` from                                                                            148 To prepare for the {0} edition you should first remove `edition = '{0}'` from
149 your `Cargo.toml` and then rerun this command. Once all warnings have been fixed                                                                         149 your `Cargo.toml` and then rerun this command. Once all warnings have been fixed
150 then you can re-enable the `edition` key in `Cargo.toml`. For some more                                                                                  150 then you can re-enable the `edition` key in `Cargo.toml`. For some more
151 information about transitioning to the {0} edition see:                                                                                                  151 information about transitioning to the {0} edition see:
152                                                                                                                                                          152 
153   https://rust-lang-nursery.github.io/edition-guide/editions/transitioning-your-code-to-a-new-edition.html                                               153   https://rust-lang-nursery.github.io/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html
154 ",                                                                                                                                                       154 ",

File_Code/cargo/53421f9ce6/diagnostic_server/diagnostic_server_after.rs --- 2/2 --- Rust
167                     "\                                                                                                                                   167                     "\
168 cannot migrate to the idioms of the {} edition for `{}`                                                                                                  168 cannot migrate to the idioms of the {} edition for `{}`
169 because it is compiled {}, which doesn't match {0}                                                                                                       169 because it is compiled {}, which doesn't match {0}
170                                                                                                                                                          170 
171 consider migrating to the {0} edition by adding `edition = '{0}'` to                                                                                     171 consider migrating to the {0} edition by adding `edition = '{0}'` to
172 `Cargo.toml` and then rerunning this command; a more detailed transition                                                                                 172 `Cargo.toml` and then rerunning this command; a more detailed transition
173 guide can be found at                                                                                                                                    173 guide can be found at
174                                                                                                                                                          174 
175   https://rust-lang-nursery.github.io/edition-guide/editions/transitioning-your-code-to-a-new-edition.html                                               175   https://rust-lang-nursery.github.io/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html
176 ",                                                                                                                                                       176 ",

