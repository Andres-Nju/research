    pub fn repair_request(&self, repair_request: &RepairType) -> Result<(SocketAddr, Vec<u8>)> {
        // find a peer that appears to be accepting replication, as indicated
        //  by a valid tvu port location
        let valid: Vec<_> = self.repair_peers();
        if valid.is_empty() {
            Err(ClusterInfoError::NoPeers)?;
        }
        let n = thread_rng().gen::<usize>() % valid.len();
        let addr = valid[n].gossip; // send the request to the peer's gossip port
        let out = {
            match repair_request {
                RepairType::Blob(slot, blob_index) => {
                    submit(
                        influxdb::Point::new("cluster_info-repair")
                            .add_field("repair-slot", influxdb::Value::Integer(*slot as i64))
                            .add_field("repair-ix", influxdb::Value::Integer(*blob_index as i64))
                            .to_owned(),
                    );
                    self.window_index_request_bytes(*slot, *blob_index)?
                }
                RepairType::HighestBlob(slot, blob_index) => {
                    submit(
                        influxdb::Point::new("cluster_info-repair_highest")
                            .add_field(
                                "repair-highest-slot",
                                influxdb::Value::Integer(*slot as i64),
                            )
                            .add_field("repair-highest-ix", influxdb::Value::Integer(*slot as i64))
                            .to_owned(),
                    );
                    self.window_highest_index_request_bytes(*slot, *blob_index)?
                }
                RepairType::Orphan(slot) => {
                    submit(
                        influxdb::Point::new("cluster_info-repair_orphan")
                            .add_field("repair-orphan", influxdb::Value::Integer(*slot as i64))
                            .to_owned(),
                    );
                    self.orphan_bytes(*slot)?
                }
            }
        };

        Ok((addr, out))
    }
