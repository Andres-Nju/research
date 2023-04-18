File_Code/solana/115d71536b/banking_stage/banking_stage_after.rs --- Rust
487             if let Err(SendPktsError::IoError(ioerr, _num_failed)) = batch_send(socket, &packet_vec)                                                     487             if let Err(SendPktsError::IoError(ioerr, num_failed)) = batch_send(socket, &packet_vec)
488             {                                                                                                                                            488             {
489                 return (Err(ioerr), 0);                                                                                                                  489                 return (Err(ioerr), packet_vec.len().saturating_sub(num_failed));

