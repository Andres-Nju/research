File_Code/nushell/f99c002426/repl/repl_after.rs --- Rust
592         r#"{}     __  ,                                                                                                                                  592         r#"{}     __  ,
593 {} .--()°'.' {}Welcome to {}Nushell{},                                                                                                                   593 {} .--()°'.' {}Welcome to {}Nushell{},
594 {}'|, . ,'   {}based on the {}nu{} language,                                                                                                             594 {}'|, . ,'   {}based on the {}nu{} language,
595 {} !_-(_\    {}where all data is structured!                                                                                                             595 {} !_-(_\    {}where all data is structured!
596                                                                                                                                                          596 
597 Please join our {}Discord{} community at {}https://discord.gg/NtAbbGn{}                                                                                  597 Please join our {}Discord{} community at {}https://discord.gg/NtAbbGn{}
598 Our {}GitHub{} repository is at {}https://github.com/nushell/nushell{}                                                                                   598 Our {}GitHub{} repository is at {}https://github.com/nushell/nushell{}
599 Our {}Documentation{} is located at {}http://nushell.sh{}                                                                                                599 Our {}Documentation{} is located at {}http://nushell.sh{}
600 {}Tweet{} us at {}@nu_shell{}                                                                                                                            600 {}Tweet{} us at {}@nu_shell{}
601                                                                                                                                                          601 
602 It's been this long since {}Nushell{}'s first commit:                                                                                                    602 It's been this long since {}Nushell{}'s first commit:
603 {}                                                                                                                                                       603 {}
604                                                                                                                                                          604 
605 {}You can disable this banner using the {}config nu{}{} command                                                                                          605 {}You can disable this banner using the {}config nu{}{} command
606 to modify the config.nu file and setting show_banner to false.                                                                                           606 to modify the config.nu file and setting show_banner to false.
607                                                                                                                                                          607 
608 let-env config {{                                                                                                                                        608 let-env config = {{
609     show_banner: false                                                                                                                                   609     show_banner: false
610     ...                                                                                                                                                  610     ...
611 }}{}                                                                                                                                                     611 }}{}
612 "#,                                                                                                                                                      612 "#,

