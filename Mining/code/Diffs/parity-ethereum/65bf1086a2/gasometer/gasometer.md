File_Code/parity-ethereum/65bf1086a2/gasometer/gasometer_after.rs --- Rust
236                                 let mem = match instruction {                                                                                            236                                 let mem = mem_needed(stack.peek(1), stack.peek(2))?;
237                                         instructions::CREATE => mem_needed(stack.peek(1), stack.peek(2))?,                                                   
238                                         instructions::CREATE2 => mem_needed(stack.peek(2), stack.peek(3))?,                                                  
239                                         _ => unreachable!("instruction can only be CREATE/CREATE2 checked above; qed"),                                      
240                                 };                                                                                                                           

