File_Code/servo/896a2de2ac/htmlimageelement/htmlimageelement_after.rs --- 1/2 --- Text (20 errors, exceeded DFT_PARSE_ERROR_LIMIT)
                                                                                                                                                           967                     // Cancel any outstanding tasks that were queued before the src was
                                                                                                                                                           968                     // set on this element.
                                                                                                                                                           969                     self.generation.set(self.generation.get() + 1);

File_Code/servo/896a2de2ac/htmlimageelement/htmlimageelement_after.rs --- 2/2 --- Text (20 errors, exceeded DFT_PARSE_ERROR_LIMIT)
974                     self.abort_request(State::CompletelyAvailable, ImageRequestPhase::Pending);                                                          977                     self.abort_request(State::Unavailable, ImageRequestPhase::Pending);

