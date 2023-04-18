File_Code/nushell/836efd237c/parse/parse_after.rs --- 1/2 --- Rust
1384                     let (internal_command, err) =                                                                                                       1384                     let (mut internal_command, err) =
1385                         parse_internal_command(&lite_cmd, registry, &signature, 1);                                                                     1385                         parse_internal_command(&lite_cmd, registry, &signature, 1);
1386                                                                                                                                                         1386 
1387                     error = error.or(err);                                                                                                              1387                     error = error.or(err);
                                                                                                                                                             1388                     internal_command.args.is_last = iter.peek().is_none();

File_Code/nushell/836efd237c/parse/parse_after.rs --- 2/2 --- Rust
1395                 let (internal_command, err) =                                                                                                           1396                 let (mut internal_command, err) =
1396                     parse_internal_command(&lite_cmd, registry, &signature, 0);                                                                         1397                     parse_internal_command(&lite_cmd, registry, &signature, 0);
1397                                                                                                                                                         1398 
1398                 error = error.or(err);                                                                                                                  1399                 error = error.or(err);
                                                                                                                                                             1400                 internal_command.args.is_last = iter.peek().is_none();

