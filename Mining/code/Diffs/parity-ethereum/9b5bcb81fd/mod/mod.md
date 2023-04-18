File_Code/parity-ethereum/9b5bcb81fd/mod/mod_after.rs --- Rust
154                 let s = self.step.read();                                                                                                                154                 let s = *self.step.read();
155                 let vote_info = message_info_rlp(&VoteStep::new(h, r, *s), block_hash);                                                                  155                 let vote_info = message_info_rlp(&VoteStep::new(h, r, s), block_hash);
156                 match self.signer.sign(vote_info.sha3()).map(Into::into) {                                                                               156                 match self.signer.sign(vote_info.sha3()).map(Into::into) {
157                         Ok(signature) => {                                                                                                               157                         Ok(signature) => {
158                                 let message_rlp = message_full_rlp(&signature, &vote_info);                                                              158                                 let message_rlp = message_full_rlp(&signature, &vote_info);
159                                 let message = ConsensusMessage::new(signature, h, r, *s, block_hash);                                                    159                                 let message = ConsensusMessage::new(signature, h, r, s, block_hash);

