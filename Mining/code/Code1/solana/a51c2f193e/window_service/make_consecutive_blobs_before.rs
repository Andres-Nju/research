    fn make_consecutive_blobs(
        me_id: Pubkey,
        mut num_blobs_to_make: u64,
        start_hash: Hash,
        addr: &SocketAddr,
        resp_recycler: &BlobRecycler,
    ) -> SharedBlobs {
        let mut msgs = Vec::new();
        let mut recorder = Recorder::new(start_hash);
        while num_blobs_to_make != 0 {
            let new_entries = recorder.record(vec![]);
            let mut new_blobs: SharedBlobs = new_entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let blob_index = num_blobs_to_make - i as u64 - 1;
                    let new_blob =
                        e.to_blob(&resp_recycler, Some(blob_index), Some(me_id), Some(addr));
                    assert_eq!(blob_index, new_blob.read().get_index().unwrap());
                    new_blob
                }).collect();
            new_blobs.truncate(num_blobs_to_make as usize);
            num_blobs_to_make -= new_blobs.len() as u64;
            msgs.extend(new_blobs);
        }
        msgs
    }
