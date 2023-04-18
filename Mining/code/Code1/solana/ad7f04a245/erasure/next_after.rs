    pub fn next(&mut self, next_data: &[SharedBlob]) -> Vec<SharedBlob> {
        let (num_data, num_coding) = self.session.dimensions();
        let mut next_coding =
            Vec::with_capacity((self.leftover.len() + next_data.len()) / num_data * num_coding);

        if !self.leftover.is_empty()
            && !next_data.is_empty()
            && self.leftover[0].read().unwrap().slot() != next_data[0].read().unwrap().slot()
        {
            self.leftover.clear();
        }

        let next_data: Vec<_> = self.leftover.iter().chain(next_data).cloned().collect();

        for data_blobs in next_data.chunks(num_data) {
            if data_blobs.len() < num_data {
                self.leftover = data_blobs.to_vec();
                break;
            }
            self.leftover.clear();

            // find max_data_size for the erasure set
            let max_data_size = data_blobs
                .iter()
                .fold(0, |max, blob| cmp::max(blob.read().unwrap().meta.size, max));

            let data_locks: Vec<_> = data_blobs.iter().map(|b| b.read().unwrap()).collect();
            let data_ptrs: Vec<_> = data_locks
                .iter()
                .map(|l| &l.data[..max_data_size])
                .collect();

            let mut coding_blobs = Vec::with_capacity(num_coding);

            for data_blob in &data_locks[..num_coding] {
                let index = data_blob.index();
                let slot = data_blob.slot();
                let id = data_blob.id();
                let genesis_blockhash = data_blob.genesis_blockhash();

                let mut coding_blob = Blob::default();
                coding_blob.set_genesis_blockhash(&genesis_blockhash);
                coding_blob.set_index(index);
                coding_blob.set_slot(slot);
                coding_blob.set_id(&id);
                coding_blob.set_size(max_data_size);
                coding_blob.set_coding();

                coding_blobs.push(coding_blob);
            }

            if {
                let mut coding_ptrs: Vec<_> = coding_blobs
                    .iter_mut()
                    .map(|blob| &mut blob.data_mut()[..max_data_size])
                    .collect();

                self.session.encode(&data_ptrs, coding_ptrs.as_mut_slice())
            }
            .is_ok()
            {
                next_coding.append(&mut coding_blobs);
            }
        }

        next_coding
            .into_iter()
            .map(|blob| Arc::new(RwLock::new(blob)))
            .collect()
    }
