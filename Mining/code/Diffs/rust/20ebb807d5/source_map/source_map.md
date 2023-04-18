File_Code/rust/20ebb807d5/source_map/source_map_after.rs --- 1/2 --- Rust
  .                                                                                                                                                          502         let hi_line = hi.line.saturating_sub(1);
502         for line_index in lo.line - 1..hi.line - 1 {                                                                                                     503         for line_index in lo.line.saturating_sub(1)..hi_line {

File_Code/rust/20ebb807d5/source_map/source_map_after.rs --- 2/2 --- Rust
509         lines.push(LineInfo { line_index: hi.line - 1, start_col, end_col: hi.col });                                                                    510         lines.push(LineInfo { line_index: hi_line, start_col, end_col: hi.col });

