File_Code/servo/0d7943765a/bluetooth/bluetooth_after.rs --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
128             if uuid_is_blacklisted(uuid.as_ref(), Blacklist::All) {                                                                                      128             if !uuid_is_blacklisted(uuid.as_ref(), Blacklist::All) {
129                 return Err(Security)                                                                                                                     129                 optional_services.push(uuid);
130             }                                                                                                                                            130             }
131             optional_services.push(uuid);                                                                                                                    

