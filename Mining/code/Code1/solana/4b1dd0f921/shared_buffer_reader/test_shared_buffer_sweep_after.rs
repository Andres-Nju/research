    fn test_shared_buffer_sweep() {
        solana_logger::setup();
        // try the inflection points with 1 to 3 readers, including a parallel reader
        // a few different chunk sizes
        for chunk_sz in [1, 2, 10] {
            // same # of buffers as default
            let equivalent_buffer_sz =
                chunk_sz * (TOTAL_BUFFER_BUDGET_DEFAULT / CHUNK_SIZE_DEFAULT);
            // 1 buffer, 2 buffers,
            for budget_sz in [
                1,
                chunk_sz,
                chunk_sz * 2,
                equivalent_buffer_sz - 1,
                equivalent_buffer_sz,
                equivalent_buffer_sz * 2,
            ] {
                for read_sz in [0, 1, chunk_sz - 1, chunk_sz, chunk_sz + 1] {
                    let read_sz = if read_sz > 0 { Some(read_sz) } else { None };
                    for reader_ct in 1..=3 {
                        for data_size in [
                            0,
                            1,
                            chunk_sz - 1,
                            chunk_sz,
                            chunk_sz + 1,
                            chunk_sz * 2 - 1,
                            chunk_sz * 2,
                            chunk_sz * 2 + 1,
                            budget_sz - 1,
                            budget_sz,
                            budget_sz + 1,
                            budget_sz * 2,
                            budget_sz * 2 - 1,
                            budget_sz * 2 + 1,
                        ] {
                            let adjusted_budget_sz = adjusted_buffer_size(budget_sz, chunk_sz);
                            let done_signal = vec![];
                            let (sender, receiver) = unbounded();
                            let file = SimpleReader::new(receiver);
                            let shared_buffer =
                                SharedBuffer::new_with_sizes(budget_sz, chunk_sz, file);
                            let mut reader = SharedBufferReader::new(&shared_buffer);
                            // with the Read trait, we don't know we are eof until we get Ok(0) from the underlying reader.
                            // This can't happen until we have enough space to store another chunk, thus we try to read another chunk and see the Ok(0) returned.
                            // Thus, we have to use data_size < adjusted_budget_sz here instead of <=
                            let second_reader = reader_ct > 1
                                && data_size < adjusted_budget_sz
                                && read_sz
                                    .as_ref()
                                    .map(|sz| sz < &adjusted_budget_sz)
                                    .unwrap_or(true);
                            let reader2 = if second_reader {
                                Some(SharedBufferReader::new(&shared_buffer))
                            } else {
                                None
                            };
                            let sent = (0..data_size)
                                .into_iter()
                                .map(|i| ((i + data_size) % 256) as u8)
                                .collect::<Vec<_>>();

                            let parallel_reader = reader_ct > 2;
                            let handle = if parallel_reader {
                                // Avoid to create more than the number of threads available in the
                                // current rayon threadpool. Deadlock could happen otherwise.
                                let threads = std::cmp::min(8, rayon::current_num_threads());
                                Some({
                                    let parallel = (0..threads)
                                        .into_iter()
                                        .map(|_| {
                                            // create before any reading starts
                                            let reader_ = SharedBufferReader::new(&shared_buffer);
                                            let sent_ = sent.clone();
                                            (reader_, sent_)
                                        })
                                        .collect::<Vec<_>>();

                                    Builder::new()
                                        .spawn(move || {
                                            parallel.into_par_iter().for_each(
                                                |(mut reader, sent)| {
                                                    let data = test_read_all(&mut reader, read_sz);
                                                    assert_eq!(
                                                        sent,
                                                        data,
                                                        "{:?}",
                                                        (
                                                            chunk_sz,
                                                            budget_sz,
                                                            read_sz,
                                                            reader_ct,
                                                            data_size,
                                                            adjusted_budget_sz
                                                        )
                                                    );
                                                },
                                            )
                                        })
                                        .unwrap()
                                })
                            } else {
                                None
                            };
                            drop(shared_buffer); // readers should work fine even if shared buffer is dropped
                            let _ = sender.send((sent.clone(), None));
                            let _ = sender.send((done_signal, None));
                            let data = test_read_all(&mut reader, read_sz);
                            assert_eq!(
                                sent,
                                data,
                                "{:?}",
                                (
                                    chunk_sz,
                                    budget_sz,
                                    read_sz,
                                    reader_ct,
                                    data_size,
                                    adjusted_budget_sz
                                )
                            );
                            // a 2nd reader would stall us if we exceed the total buffer size
                            if second_reader {
                                // #2 will read valid bytes first and succeed, then get error
                                let data = test_read_all(&mut reader2.unwrap(), read_sz);
                                assert_eq!(sent, data);
                            }
                            if parallel_reader {
                                assert!(handle.unwrap().join().is_ok());
                            }
                        }
                    }
                }
            }
        }
    }
