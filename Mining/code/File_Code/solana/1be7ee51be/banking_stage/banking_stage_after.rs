//! The `banking_stage` processes Transaction messages. It is intended to be used
//! to contruct a software pipeline. The stage uses all available CPU cores and
//! can do its processing in parallel with signature verification on the GPU.

use crate::cluster_info::ClusterInfo;
use crate::entry;
use crate::entry::Entry;
use crate::leader_confirmation_service::LeaderConfirmationService;
use crate::leader_schedule_utils;
use crate::packet;
use crate::packet::SharedPackets;
use crate::packet::{Packet, Packets};
use crate::poh_recorder::{PohRecorder, PohRecorderError, WorkingBankEntries};
use crate::poh_service::{PohService, PohServiceConfig};
use crate::result::{Error, Result};
use crate::service::Service;
use crate::sigverify_stage::VerifiedPackets;
use bincode::deserialize;
use solana_metrics::counter::Counter;
use solana_runtime::bank::{self, Bank};
use solana_sdk::timing::{self, duration_as_us, MAX_RECENT_BLOCKHASHES};
use solana_sdk::transaction::{Transaction, TransactionError};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, Builder, JoinHandle};
use std::time::Duration;
use std::time::Instant;
use sys_info;

pub type UnprocessedPackets = Vec<(SharedPackets, usize)>; // `usize` is the index of the first unprocessed packet in `SharedPackets`

// number of threads is 1 until mt bank is ready
pub const NUM_THREADS: u32 = 10;

/// Stores the stage's thread handle and output receiver.
pub struct BankingStage {
    bank_thread_hdls: Vec<JoinHandle<()>>,
}

impl BankingStage {
    /// Create the stage using `bank`. Exit when `verified_receiver` is dropped.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        cluster_info: &Arc<RwLock<ClusterInfo>>,
        poh_recorder: &Arc<Mutex<PohRecorder>>,
        verified_receiver: Receiver<VerifiedPackets>,
    ) -> Self {
        Self::new_num_threads(
            cluster_info,
            poh_recorder,
            verified_receiver,
            Self::num_threads(),
        )
    }

    pub fn new_num_threads(
        cluster_info: &Arc<RwLock<ClusterInfo>>,
        poh_recorder: &Arc<Mutex<PohRecorder>>,
        verified_receiver: Receiver<VerifiedPackets>,
        num_threads: u32,
    ) -> Self {
        let verified_receiver = Arc::new(Mutex::new(verified_receiver));

        // Single thread to generate entries from many banks.
        // This thread talks to poh_service and broadcasts the entries once they have been recorded.
        // Once an entry has been recorded, its blockhash is registered with the bank.
        let exit = Arc::new(AtomicBool::new(false));

        // Single thread to compute confirmation
        let lcs_handle = LeaderConfirmationService::start(&poh_recorder, exit.clone());
        // Many banks that process transactions in parallel.
        let mut bank_thread_hdls: Vec<JoinHandle<()>> = (0..num_threads)
            .map(|_| {
                let verified_receiver = verified_receiver.clone();
                let poh_recorder = poh_recorder.clone();
                let cluster_info = cluster_info.clone();
                let exit = exit.clone();
                Builder::new()
                    .name("solana-banking-stage-tx".to_string())
                    .spawn(move || {
                        Self::process_loop(&verified_receiver, &poh_recorder, &cluster_info);
                        exit.store(true, Ordering::Relaxed);
                    })
                    .unwrap()
            })
            .collect();
        bank_thread_hdls.push(lcs_handle);
        Self { bank_thread_hdls }
    }

    fn forward_unprocessed_packets(
        socket: &std::net::UdpSocket,
        tpu_via_blobs: &std::net::SocketAddr,
        unprocessed_packets: &[(SharedPackets, usize)],
    ) -> std::io::Result<()> {
        let locked_packets: Vec<_> = unprocessed_packets
            .iter()
            .map(|(p, start_index)| (p.read().unwrap(), start_index))
            .collect();
        let packets: Vec<&Packet> = locked_packets
            .iter()
            .flat_map(|(p, start_index)| &p.packets[**start_index..])
            .collect();
        let blobs = packet::packets_to_blobs(&packets);

        for blob in blobs {
            socket.send_to(&blob.data[..blob.meta.size], tpu_via_blobs)?;
        }

        Ok(())
    }

    fn forward_buffered_packets(
        socket: &std::net::UdpSocket,
        poh_recorder: &Arc<Mutex<PohRecorder>>,
        cluster_info: &Arc<RwLock<ClusterInfo>>,
        buffered_packets: &[(SharedPackets, usize)],
    ) -> bool {
        let rcluster_info = cluster_info.read().unwrap();

        // If there's a bank, and leader is available, forward the buffered packets
        // or, if the current node is not the leader, forward the buffered packets
        let forward = match poh_recorder.lock().unwrap().bank() {
            Some(_) => rcluster_info.leader_data().is_some(),
            None => rcluster_info
                .leader_data()
                .map(|x| x.id != rcluster_info.id())
                .unwrap_or(false),
        };

        if forward {
            let _ = Self::forward_unprocessed_packets(
                &socket,
                &rcluster_info.leader_data().unwrap().tpu_via_blobs,
                &buffered_packets,
            );
        }

        forward
    }

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

    pub fn process_loop(
        verified_receiver: &Arc<Mutex<Receiver<VerifiedPackets>>>,
        poh_recorder: &Arc<Mutex<PohRecorder>>,
        cluster_info: &Arc<RwLock<ClusterInfo>>,
    ) {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        let mut buffered_packets = vec![];
        loop {
            if Self::forward_buffered_packets(
                &socket,
                poh_recorder,
                cluster_info,
                &buffered_packets,
            ) {
                buffered_packets.clear();
            }

            match Self::process_packets(&verified_receiver, &poh_recorder) {
                Err(Error::RecvTimeoutError(RecvTimeoutError::Timeout)) => (),
                Ok(unprocessed_packets) => {
                    if Self::should_buffer_packets(poh_recorder, cluster_info) {
                        buffered_packets.extend_from_slice(&unprocessed_packets);
                        continue;
                    }

                    if let Some(leader) = cluster_info.read().unwrap().leader_data() {
                        let _ = Self::forward_unprocessed_packets(
                            &socket,
                            &leader.tpu_via_blobs,
                            &unprocessed_packets,
                        );
                    }
                }
                Err(err) => {
                    debug!("solana-banking-stage-tx: exit due to {:?}", err);
                    break;
                }
            }
        }
    }

    pub fn num_threads() -> u32 {
        sys_info::cpu_num().unwrap_or(NUM_THREADS)
    }

    /// Convert the transactions from a blob of binary data to a vector of transactions
    fn deserialize_transactions(p: &Packets) -> Vec<Option<Transaction>> {
        p.packets
            .iter()
            .map(|x| deserialize(&x.data[0..x.meta.size]).ok())
            .collect()
    }

    fn record_transactions(
        txs: &[Transaction],
        results: &[bank::Result<()>],
        poh: &Arc<Mutex<PohRecorder>>,
    ) -> Result<()> {
        let processed_transactions: Vec<_> = results
            .iter()
            .zip(txs.iter())
            .filter_map(|(r, x)| match r {
                Ok(_) => Some(x.clone()),
                Err(TransactionError::InstructionError(index, err)) => {
                    info!("instruction error {:?}, {:?}", index, err);
                    Some(x.clone())
                }
                Err(ref e) => {
                    debug!("process transaction failed {:?}", e);
                    None
                }
            })
            .collect();
        debug!("processed: {} ", processed_transactions.len());
        // unlock all the accounts with errors which are filtered by the above `filter_map`
        if !processed_transactions.is_empty() {
            let hash = Transaction::hash(&processed_transactions);
            // record and unlock will unlock all the successfull transactions
            poh.lock().unwrap().record(hash, processed_transactions)?;
        }
        Ok(())
    }

    pub fn process_and_record_transactions(
        bank: &Bank,
        txs: &[Transaction],
        poh: &Arc<Mutex<PohRecorder>>,
    ) -> Result<()> {
        let now = Instant::now();
        // Once accounts are locked, other threads cannot encode transactions that will modify the
        // same account state
        let lock_results = bank.lock_accounts(txs);
        let lock_time = now.elapsed();

        let now = Instant::now();
        // Use a shorter maximum age when adding transactions into the pipeline.  This will reduce
        // the likelihood of any single thread getting starved and processing old ids.
        // TODO: Banking stage threads should be prioritized to complete faster then this queue
        // expires.
        let (loaded_accounts, results) =
            bank.load_and_execute_transactions(txs, lock_results, MAX_RECENT_BLOCKHASHES / 2);
        let load_execute_time = now.elapsed();

        let record_time = {
            let now = Instant::now();
            Self::record_transactions(txs, &results, poh)?;
            now.elapsed()
        };

        let commit_time = {
            let now = Instant::now();
            bank.commit_transactions(txs, &loaded_accounts, &results);
            now.elapsed()
        };

        let now = Instant::now();
        // Once the accounts are new transactions can enter the pipeline to process them
        bank.unlock_accounts(&txs, &results);
        let unlock_time = now.elapsed();
        debug!(
            "bank: {} lock: {}us load_execute: {}us record: {}us commit: {}us unlock: {}us txs_len: {}",
            bank.slot(),
            duration_as_us(&lock_time),
            duration_as_us(&load_execute_time),
            duration_as_us(&record_time),
            duration_as_us(&commit_time),
            duration_as_us(&unlock_time),
            txs.len(),
        );
        Ok(())
    }

    /// Sends transactions to the bank.
    ///
    /// Returns the number of transactions successfully processed by the bank, which may be less
    /// than the total number if max PoH height was reached and the bank halted
    fn process_transactions(
        bank: &Bank,
        transactions: &[Transaction],
        poh: &Arc<Mutex<PohRecorder>>,
    ) -> Result<(usize)> {
        let mut chunk_start = 0;
        while chunk_start != transactions.len() {
            let chunk_end = chunk_start
                + entry::num_will_fit(
                    &transactions[chunk_start..],
                    packet::BLOB_DATA_SIZE as u64,
                    &Entry::serialized_size,
                );

            let result = Self::process_and_record_transactions(
                bank,
                &transactions[chunk_start..chunk_end],
                poh,
            );
            trace!("process_transcations: {:?}", result);
            if let Err(Error::PohRecorderError(PohRecorderError::MaxHeightReached)) = result {
                info!(
                    "process transactions: max height reached slot: {} height: {}",
                    bank.slot(),
                    bank.tick_height()
                );
                break;
            }
            result?;
            chunk_start = chunk_end;
        }
        Ok(chunk_start)
    }

    /// Process the incoming packets
    pub fn process_packets(
        verified_receiver: &Arc<Mutex<Receiver<VerifiedPackets>>>,
        poh: &Arc<Mutex<PohRecorder>>,
    ) -> Result<UnprocessedPackets> {
        let recv_start = Instant::now();
        let mms = verified_receiver
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_millis(100))?;

        let mut reqs_len = 0;
        let mms_len = mms.len();
        info!(
            "@{:?} process start stalled for: {:?}ms batches: {}",
            timing::timestamp(),
            timing::duration_as_ms(&recv_start.elapsed()),
            mms.len(),
        );
        inc_new_counter_info!("banking_stage-entries_received", mms_len);
        let count = mms.iter().map(|x| x.1.len()).sum();
        let proc_start = Instant::now();
        let mut new_tx_count = 0;

        let mut unprocessed_packets = vec![];
        let mut bank_shutdown = false;
        for (msgs, vers) in mms {
            if bank_shutdown {
                unprocessed_packets.push((msgs, 0));
                continue;
            }

            let bank = poh.lock().unwrap().bank();
            if bank.is_none() {
                unprocessed_packets.push((msgs, 0));
                continue;
            }
            let bank = bank.unwrap();
            debug!("banking-stage-tx bank {}", bank.slot());

            let transactions = Self::deserialize_transactions(&msgs.read().unwrap());
            reqs_len += transactions.len();

            debug!(
                "bank: {} transactions received {}",
                bank.slot(),
                transactions.len()
            );
            let (verified_transactions, verified_transaction_index): (Vec<_>, Vec<_>) =
                transactions
                    .into_iter()
                    .zip(vers)
                    .zip(0..)
                    .filter_map(|((tx, ver), index)| match tx {
                        None => None,
                        Some(tx) => {
                            if tx.verify_refs() && ver != 0 {
                                Some((tx, index))
                            } else {
                                None
                            }
                        }
                    })
                    .unzip();

            debug!(
                "bank: {} verified transactions {}",
                bank.slot(),
                verified_transactions.len()
            );

            let processed = Self::process_transactions(&bank, &verified_transactions, poh)?;
            if processed < verified_transactions.len() {
                bank_shutdown = true;
                // Collect any unprocessed transactions in this batch for forwarding
                unprocessed_packets.push((msgs, verified_transaction_index[processed]));
            }
            new_tx_count += processed;
        }

        inc_new_counter_info!(
            "banking_stage-time_ms",
            timing::duration_as_ms(&proc_start.elapsed()) as usize
        );
        let total_time_s = timing::duration_as_s(&proc_start.elapsed());
        let total_time_ms = timing::duration_as_ms(&proc_start.elapsed());
        info!(
            "@{:?} done processing transaction batches: {} time: {:?}ms reqs: {} reqs/s: {}",
            timing::timestamp(),
            mms_len,
            total_time_ms,
            reqs_len,
            (reqs_len as f32) / (total_time_s)
        );
        inc_new_counter_info!("banking_stage-process_packets", count);
        inc_new_counter_info!("banking_stage-process_transactions", new_tx_count);

        Ok(unprocessed_packets)
    }
}

impl Service for BankingStage {
    type JoinReturnType = ();

    fn join(self) -> thread::Result<()> {
        for bank_thread_hdl in self.bank_thread_hdls {
            bank_thread_hdl.join()?;
        }
        Ok(())
    }
}

pub fn create_test_recorder(
    bank: &Arc<Bank>,
) -> (
    Arc<AtomicBool>,
    Arc<Mutex<PohRecorder>>,
    PohService,
    Receiver<WorkingBankEntries>,
) {
    let exit = Arc::new(AtomicBool::new(false));
    let (mut poh_recorder, entry_receiver) = PohRecorder::new(
        bank.tick_height(),
        bank.last_blockhash(),
        bank.slot(),
        Some(4),
        bank.ticks_per_slot(),
    );
    poh_recorder.set_bank(&bank);

    let poh_recorder = Arc::new(Mutex::new(poh_recorder));
    let poh_service = PohService::new(poh_recorder.clone(), &PohServiceConfig::default(), &exit);

    (exit, poh_recorder, poh_service, entry_receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster_info::Node;
    use crate::entry::EntrySlice;
    use crate::packet::to_packets;
    use crate::poh_recorder::WorkingBank;
    use solana_sdk::genesis_block::GenesisBlock;
    use solana_sdk::signature::{Keypair, KeypairUtil};
    use solana_sdk::system_transaction::SystemTransaction;
    use solana_sdk::transaction::InstructionError;
    use std::sync::mpsc::channel;
    use std::thread::sleep;

    #[test]
    fn test_banking_stage_shutdown1() {
        let (genesis_block, _mint_keypair) = GenesisBlock::new(2);
        let bank = Arc::new(Bank::new(&genesis_block));
        let (verified_sender, verified_receiver) = channel();
        let (exit, poh_recorder, poh_service, _entry_receiever) = create_test_recorder(&bank);
        let cluster_info = ClusterInfo::new_with_invalid_keypair(Node::new_localhost().info);
        let cluster_info = Arc::new(RwLock::new(cluster_info));
        let banking_stage = BankingStage::new(&cluster_info, &poh_recorder, verified_receiver);
        drop(verified_sender);
        exit.store(true, Ordering::Relaxed);
        banking_stage.join().unwrap();
        poh_service.join().unwrap();
    }

    #[test]
    fn test_banking_stage_tick() {
        solana_logger::setup();
        let (mut genesis_block, _mint_keypair) = GenesisBlock::new(2);
        genesis_block.ticks_per_slot = 4;
        let bank = Arc::new(Bank::new(&genesis_block));
        let start_hash = bank.last_blockhash();
        let (verified_sender, verified_receiver) = channel();
        let (exit, poh_recorder, poh_service, entry_receiver) = create_test_recorder(&bank);
        let cluster_info = ClusterInfo::new_with_invalid_keypair(Node::new_localhost().info);
        let cluster_info = Arc::new(RwLock::new(cluster_info));
        let banking_stage = BankingStage::new(&cluster_info, &poh_recorder, verified_receiver);
        trace!("sending bank");
        sleep(Duration::from_millis(600));
        drop(verified_sender);
        exit.store(true, Ordering::Relaxed);
        poh_service.join().unwrap();
        drop(poh_recorder);

        trace!("getting entries");
        let entries: Vec<_> = entry_receiver
            .iter()
            .flat_map(|x| x.1.into_iter().map(|e| e.0))
            .collect();
        trace!("done");
        assert_eq!(entries.len(), genesis_block.ticks_per_slot as usize - 1);
        assert!(entries.verify(&start_hash));
        assert_eq!(entries[entries.len() - 1].hash, bank.last_blockhash());
        banking_stage.join().unwrap();
    }

    #[test]
    fn test_banking_stage_entries_only() {
        solana_logger::setup();
        let (genesis_block, mint_keypair) = GenesisBlock::new(10);
        let bank = Arc::new(Bank::new(&genesis_block));
        let start_hash = bank.last_blockhash();
        let (verified_sender, verified_receiver) = channel();
        let (exit, poh_recorder, poh_service, entry_receiver) = create_test_recorder(&bank);
        let cluster_info = ClusterInfo::new_with_invalid_keypair(Node::new_localhost().info);
        let cluster_info = Arc::new(RwLock::new(cluster_info));
        let banking_stage = BankingStage::new(&cluster_info, &poh_recorder, verified_receiver);

        // fund another account so we can send 2 good transactions in a single batch.
        let keypair = Keypair::new();
        let fund_tx =
            SystemTransaction::new_account(&mint_keypair, &keypair.pubkey(), 2, start_hash, 0);
        bank.process_transaction(&fund_tx).unwrap();

        // good tx
        let to = Keypair::new().pubkey();
        let tx = SystemTransaction::new_account(&mint_keypair, &to, 1, start_hash, 0);

        // good tx, but no verify
        let to2 = Keypair::new().pubkey();
        let tx_no_ver = SystemTransaction::new_account(&keypair, &to2, 2, start_hash, 0);

        // bad tx, AccountNotFound
        let keypair = Keypair::new();
        let to3 = Keypair::new().pubkey();
        let tx_anf = SystemTransaction::new_account(&keypair, &to3, 1, start_hash, 0);

        // send 'em over
        let packets = to_packets(&[tx_no_ver, tx_anf, tx]);

        // glad they all fit
        assert_eq!(packets.len(), 1);
        verified_sender // no_ver, anf, tx
            .send(vec![(packets[0].clone(), vec![0u8, 1u8, 1u8])])
            .unwrap();

        drop(verified_sender);
        exit.store(true, Ordering::Relaxed);
        poh_service.join().unwrap();
        drop(poh_recorder);

        let mut blockhash = start_hash;
        let bank = Bank::new(&genesis_block);
        bank.process_transaction(&fund_tx).unwrap();
        //receive entries + ticks
        for _ in 0..10 {
            let ventries: Vec<Vec<Entry>> = entry_receiver
                .iter()
                .map(|x| x.1.into_iter().map(|e| e.0).collect())
                .collect();

            for entries in &ventries {
                for entry in entries {
                    bank.process_transactions(&entry.transactions)
                        .iter()
                        .for_each(|x| assert_eq!(*x, Ok(())));
                }
                assert!(entries.verify(&blockhash));
                blockhash = entries.last().unwrap().hash;
            }

            if bank.get_balance(&to) == 1 {
                break;
            }

            sleep(Duration::from_millis(200));
        }

        assert_eq!(bank.get_balance(&to), 1);
        assert_eq!(bank.get_balance(&to2), 0);

        drop(entry_receiver);
        banking_stage.join().unwrap();
    }

    #[test]
    fn test_banking_stage_entryfication() {
        solana_logger::setup();
        // In this attack we'll demonstrate that a verifier can interpret the ledger
        // differently if either the server doesn't signal the ledger to add an
        // Entry OR if the verifier tries to parallelize across multiple Entries.
        let (genesis_block, mint_keypair) = GenesisBlock::new(2);
        let (verified_sender, verified_receiver) = channel();

        // Process a batch that includes a transaction that receives two lamports.
        let alice = Keypair::new();
        let tx = SystemTransaction::new_account(
            &mint_keypair,
            &alice.pubkey(),
            2,
            genesis_block.hash(),
            0,
        );

        let packets = to_packets(&[tx]);
        verified_sender
            .send(vec![(packets[0].clone(), vec![1u8])])
            .unwrap();

        // Process a second batch that spends one of those lamports.
        let tx = SystemTransaction::new_account(
            &alice,
            &mint_keypair.pubkey(),
            1,
            genesis_block.hash(),
            0,
        );
        let packets = to_packets(&[tx]);
        verified_sender
            .send(vec![(packets[0].clone(), vec![1u8])])
            .unwrap();

        let entry_receiver = {
            // start a banking_stage to eat verified receiver
            let bank = Arc::new(Bank::new(&genesis_block));
            let (exit, poh_recorder, poh_service, entry_receiver) = create_test_recorder(&bank);
            let cluster_info = ClusterInfo::new_with_invalid_keypair(Node::new_localhost().info);
            let cluster_info = Arc::new(RwLock::new(cluster_info));
            let _banking_stage =
                BankingStage::new_num_threads(&cluster_info, &poh_recorder, verified_receiver, 1);

            // wait for banking_stage to eat the packets
            while bank.get_balance(&alice.pubkey()) != 1 {
                sleep(Duration::from_millis(100));
            }
            exit.store(true, Ordering::Relaxed);
            poh_service.join().unwrap();
            entry_receiver
        };
        drop(verified_sender);

        // consume the entire entry_receiver, feed it into a new bank
        // check that the balance is what we expect.
        let entries: Vec<_> = entry_receiver
            .iter()
            .flat_map(|x| x.1.into_iter().map(|e| e.0))
            .collect();

        let bank = Bank::new(&genesis_block);
        for entry in &entries {
            bank.process_transactions(&entry.transactions)
                .iter()
                .for_each(|x| assert_eq!(*x, Ok(())));
        }

        // Assert the user holds one lamport, not two. If the stage only outputs one
        // entry, then the second transaction will be rejected, because it drives
        // the account balance below zero before the credit is added.
        assert_eq!(bank.get_balance(&alice.pubkey()), 1);
    }

    #[test]
    fn test_bank_record_transactions() {
        let (genesis_block, mint_keypair) = GenesisBlock::new(10_000);
        let bank = Arc::new(Bank::new(&genesis_block));
        let working_bank = WorkingBank {
            bank: bank.clone(),
            min_tick_height: bank.tick_height(),
            max_tick_height: std::u64::MAX,
        };

        let (poh_recorder, entry_receiver) = PohRecorder::new(
            bank.tick_height(),
            bank.last_blockhash(),
            bank.slot(),
            None,
            bank.ticks_per_slot(),
        );
        let poh_recorder = Arc::new(Mutex::new(poh_recorder));

        poh_recorder.lock().unwrap().set_working_bank(working_bank);
        let pubkey = Keypair::new().pubkey();

        let transactions = vec![
            SystemTransaction::new_move(&mint_keypair, &pubkey, 1, genesis_block.hash(), 0),
            SystemTransaction::new_move(&mint_keypair, &pubkey, 1, genesis_block.hash(), 0),
        ];

        let mut results = vec![Ok(()), Ok(())];
        BankingStage::record_transactions(&transactions, &results, &poh_recorder).unwrap();
        let (_, entries) = entry_receiver.recv().unwrap();
        assert_eq!(entries[0].0.transactions.len(), transactions.len());

        // InstructionErrors should still be recorded
        results[0] = Err(TransactionError::InstructionError(
            1,
            InstructionError::new_result_with_negative_lamports(),
        ));
        BankingStage::record_transactions(&transactions, &results, &poh_recorder).unwrap();
        let (_, entries) = entry_receiver.recv().unwrap();
        assert_eq!(entries[0].0.transactions.len(), transactions.len());

        // Other TransactionErrors should not be recorded
        results[0] = Err(TransactionError::AccountNotFound);
        BankingStage::record_transactions(&transactions, &results, &poh_recorder).unwrap();
        let (_, entries) = entry_receiver.recv().unwrap();
        assert_eq!(entries[0].0.transactions.len(), transactions.len() - 1);
    }

    #[test]
    fn test_bank_process_and_record_transactions() {
        solana_logger::setup();
        let (genesis_block, mint_keypair) = GenesisBlock::new(10_000);
        let bank = Arc::new(Bank::new(&genesis_block));
        let pubkey = Keypair::new().pubkey();

        let transactions = vec![SystemTransaction::new_move(
            &mint_keypair,
            &pubkey,
            1,
            genesis_block.hash(),
            0,
        )];

        let working_bank = WorkingBank {
            bank: bank.clone(),
            min_tick_height: bank.tick_height(),
            max_tick_height: bank.tick_height() + 1,
        };
        let (poh_recorder, entry_receiver) = PohRecorder::new(
            bank.tick_height(),
            bank.last_blockhash(),
            bank.slot(),
            Some(4),
            bank.ticks_per_slot(),
        );
        let poh_recorder = Arc::new(Mutex::new(poh_recorder));

        poh_recorder.lock().unwrap().set_working_bank(working_bank);

        BankingStage::process_and_record_transactions(&bank, &transactions, &poh_recorder).unwrap();
        poh_recorder.lock().unwrap().tick();

        let mut done = false;
        // read entries until I find mine, might be ticks...
        while let Ok((_, entries)) = entry_receiver.recv() {
            for (entry, _) in entries {
                if !entry.is_tick() {
                    trace!("got entry");
                    assert_eq!(entry.transactions.len(), transactions.len());
                    assert_eq!(bank.get_balance(&pubkey), 1);
                    done = true;
                }
            }
            if done {
                break;
            }
        }
        trace!("done ticking");

        assert_eq!(done, true);

        let transactions = vec![SystemTransaction::new_move(
            &mint_keypair,
            &pubkey,
            2,
            genesis_block.hash(),
            0,
        )];

        assert_matches!(
            BankingStage::process_and_record_transactions(&bank, &transactions, &poh_recorder),
            Err(Error::PohRecorderError(PohRecorderError::MaxHeightReached))
        );

        assert_eq!(bank.get_balance(&pubkey), 1);
    }
}
