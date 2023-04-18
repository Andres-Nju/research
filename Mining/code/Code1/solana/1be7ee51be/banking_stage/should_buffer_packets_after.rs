    fn should_buffer_packets(
        poh_recorder: &Arc<Mutex<PohRecorder>>,
        cluster_info: &Arc<RwLock<ClusterInfo>>,
    ) -> bool {
        let rcluster_info = cluster_info.read().unwrap();

        // Buffer the packets if I am the next leader
        // or, if it was getting sent to me
        let leader_id = match poh_recorder.lock().unwrap().bank() {
            Some(bank) => {
                leader_schedule_utils::slot_leader_at(bank.slot() + 1, &bank).unwrap_or_default()
            }
            None => rcluster_info
                .leader_data()
                .map(|x| x.id)
                .unwrap_or_default(),
        };

        leader_id == rcluster_info.id()
    }
