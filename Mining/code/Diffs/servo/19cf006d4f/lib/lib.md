File_Code/servo/19cf006d4f/lib/lib_after.rs --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
231         if response.is_network_error() {                                                                                                                 231         if let Some(e) = response.get_network_error() {
232             // todo: finer grained errors                                                                                                                232             let _ = self.send(FetchResponseMsg::ProcessResponseEOF(Err(e.clone())));
233             let _ =                                                                                                                                          
234                 self.send(FetchResponseMsg::ProcessResponseEOF(Err(NetworkError::Internal("Network error".into()))));                                        

