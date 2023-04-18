File_Code/solana/1b5845ac3e/cluster_info/cluster_info_after.rs --- 1/2 --- Rust
297     /// * since - The local timestamp when the vote was updated or inserted must be greater then                                                         297     /// * since - The timestamp of when the vote inserted must be greater than
298     /// since. This allows the bank to query for new votes only.                                                                                         298     /// since. This allows the bank to query for new votes only.
299     ///                                                                                                                                                  299     ///
300     /// * return - The votes, and the max local timestamp from the new set.                                                                              300     /// * return - The votes, and the max timestamp from the new set.

File_Code/solana/1b5845ac3e/cluster_info/cluster_info_after.rs --- 2/2 --- Rust
307             .filter(|x| x.local_timestamp > since)                                                                                                       307             .filter(|x| x.insert_timestamp > since)
308             .filter_map(|x| {                                                                                                                            308             .filter_map(|x| {
309                 x.value                                                                                                                                  309                 x.value
310                     .vote()                                                                                                                              310                     .vote()
311                     .map(|v| (x.local_timestamp, v.transaction.clone()))                                                                                 311                     .map(|v| (x.insert_timestamp, v.transaction.clone()))

