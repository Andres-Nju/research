File_Code/parity-ethereum/91f864b2b7/block/block_after.rs --- 1/2 --- Rust
                                                                                                                                                            19 use std::cmp;

File_Code/parity-ethereum/91f864b2b7/block/block_after.rs --- 2/2 --- Rust
269                 let gas_floor_target = ::std::cmp::max(gas_range_target.0, engine.params().min_gas_limit);                                               270                 let gas_floor_target = cmp::max(gas_range_target.0, engine.params().min_gas_limit);
...                                                                                                                                                          271                 let gas_ceil_target = cmp::max(gas_range_target.1, gas_floor_target);
270                 engine.populate_from_parent(&mut r.block.base.header, parent, gas_floor_target, gas_range_target.1);                                     272                 engine.populate_from_parent(&mut r.block.base.header, parent, gas_floor_target, gas_ceil_target);

