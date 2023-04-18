File_Code/solana/f4ca87205f/postgres_client/postgres_client_after.rs --- Rust
673                         self.client.log_transaction(*transaction_log_info)?;                                                                             673                         if let Err(err) = self.client.log_transaction(*transaction_log_info) {
                                                                                                                                                             674                             error!("Failed to update transaction: ({})", err);
                                                                                                                                                             675                             if panic_on_db_errors {
                                                                                                                                                             676                                 abort();
                                                                                                                                                             677                             }
                                                                                                                                                             678                         }

