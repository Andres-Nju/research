    fn next(&mut self) -> Option<Self::Item> {
        let mut batch = vec![];

        let start_time = Instant::now();

        let mut idx = 0;

        // Pull from stream until time runs out or we have enough items
        for item in self.stream.by_ref() {
            batch.push(item);
            idx += 1;

            if idx % STREAM_TIMEOUT_CHECK_INTERVAL == 0 {
                let end_time = Instant::now();

                // If we've been buffering over a second, go ahead and send out what we have so far
                if (end_time - start_time).as_secs() >= 1 {
                    break;
                }
            }

            if idx == STREAM_PAGE_SIZE {
                break;
            }

            if let Some(ctrlc) = &self.ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    break;
                }
            }
        }

        let table = match &self.view {
            TableView::General => self.build_general(&batch),
            TableView::Collapsed => self.build_collapsed(batch),
            TableView::Expanded {
                limit,
                flatten,
                flatten_separator,
            } => self.build_extended(&batch, *limit, *flatten, flatten_separator.clone()),
        };

        self.row_offset += idx;

        match table {
            Ok(Some(table)) => Some(Ok(table.as_bytes().to_vec())),
            Err(err) => Some(Err(err)),
            _ => None,
        }
    }
