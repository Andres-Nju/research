    fn do_work(
        &mut self,
        receiver: Receiver<DbWorkItem>,
        exit_worker: Arc<AtomicBool>,
        is_startup_done: Arc<AtomicBool>,
        startup_done_count: Arc<AtomicUsize>,
        panic_on_db_errors: bool,
    ) -> Result<(), AccountsDbPluginError> {
        while !exit_worker.load(Ordering::Relaxed) {
            let mut measure = Measure::start("accountsdb-plugin-postgres-worker-recv");
            let work = receiver.recv_timeout(Duration::from_millis(500));
            measure.stop();
            inc_new_counter_debug!(
                "accountsdb-plugin-postgres-worker-recv-us",
                measure.as_us() as usize,
                100000,
                100000
            );
            match work {
                Ok(work) => match work {
                    DbWorkItem::UpdateAccount(request) => {
                        if let Err(err) = self
                            .client
                            .update_account(request.account, request.is_startup)
                        {
                            error!("Failed to update account: ({})", err);
                            if panic_on_db_errors {
                                abort();
                            }
                        }
                    }
                    DbWorkItem::UpdateSlot(request) => {
                        if let Err(err) = self.client.update_slot_status(
                            request.slot,
                            request.parent,
                            request.slot_status,
                        ) {
                            error!("Failed to update slot: ({})", err);
                            if panic_on_db_errors {
                                abort();
                            }
                        }
                    }
                    DbWorkItem::LogTransaction(transaction_log_info) => {
                        self.client.log_transaction(*transaction_log_info)?;
                    }
                },
                Err(err) => match err {
                    RecvTimeoutError::Timeout => {
                        if !self.is_startup_done && is_startup_done.load(Ordering::Relaxed) {
                            if let Err(err) = self.client.notify_end_of_startup() {
                                error!("Error in notifying end of startup: ({})", err);
                                if panic_on_db_errors {
                                    abort();
                                }
                            }
                            self.is_startup_done = true;
                            startup_done_count.fetch_add(1, Ordering::Relaxed);
                        }

                        continue;
                    }
                    _ => {
                        error!("Error in receiving the item {:?}", err);
                        if panic_on_db_errors {
                            abort();
                        }
                        break;
                    }
                },
            }
        }
        Ok(())
    }
