File_Code/solana/a369b4a070/tpu_client/tpu_client_after.rs --- 1/2 --- Rust
148     pub fn get_leader_sockets(&self, current_slot: Slot, fanout_slots: u64) -> Vec<SocketAddr> {                                                         148     fn get_leader_sockets(&self, fanout_slots: u64) -> Vec<SocketAddr> {
149         let mut leader_set = HashSet::new();                                                                                                             149         let mut leader_set = HashSet::new();
150         let mut leader_sockets = Vec::new();                                                                                                             150         let mut leader_sockets = Vec::new();
151         for leader_slot in current_slot..current_slot + fanout_slots {                                                                                   151         for leader_slot in self.first_slot..self.first_slot + fanout_slots {

File_Code/solana/a369b4a070/tpu_client/tpu_client_after.rs --- 2/2 --- Rust
631         let current_slot = self.recent_slots.estimated_current_slot();                                                                                   ... 
632         self.leader_tpu_cache                                                                                                                            631         self.leader_tpu_cache
633             .read()                                                                                                                                      632             .read()
634             .unwrap()                                                                                                                                    633             .unwrap()
635             .get_leader_sockets(current_slot, fanout_slots)                                                                                              634             .get_leader_sockets(fanout_slots)

