    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Packet {{ neighborhood_bounds: {:?}, current_layer: {:?}, child_layer_bounds: {:?} child_layer_peers: {:?} }}",
            self.neighbor_bounds, self.layer_ix, self.child_layer_bounds, self.child_layer_peers
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PruneData {
    /// Pubkey of the node that sent this prune data
    pub pubkey: Pubkey,
    /// Pubkeys of nodes that should be pruned
    pub prunes: Vec<Pubkey>,
    /// Signature of this Prune Message
    pub signature: Signature,
    /// The Pubkey of the intended node/destination for this message
    pub destination: Pubkey,
    /// Wallclock of the node that generated this message
    pub wallclock: u64,
}

impl Signable for PruneData {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    fn signable_data(&self) -> Vec<u8> {
        #[derive(Serialize)]
        struct SignData {
            pubkey: Pubkey,
            prunes: Vec<Pubkey>,
            destination: Pubkey,
            wallclock: u64,
        }
        let data = SignData {
            pubkey: self.pubkey,
            prunes: self.prunes.clone(),
            destination: self.destination,
            wallclock: self.wallclock,
        };
        serialize(&data).expect("serialize PruneData")
    }

    fn get_signature(&self) -> Signature {
        self.signature
    }

    fn set_signature(&mut self, signature: Signature) {
        self.signature = signature
    }
}

// TODO These messages should go through the gpu pipeline for spam filtering
#[derive(Serialize, Deserialize, Debug)]
#[allow(clippy::large_enum_variant)]
enum Protocol {
    /// Gossip protocol messages
    PullRequest(Bloom<Hash>, CrdsValue),
    PullResponse(Pubkey, Vec<CrdsValue>),
    PushMessage(Pubkey, Vec<CrdsValue>),
    PruneMessage(Pubkey, PruneData),

    /// Window protocol messages
    /// TODO: move this message to a different module
    RequestWindowIndex(ContactInfo, u64, u64),
    RequestHighestWindowIndex(ContactInfo, u64, u64),
    RequestOrphan(ContactInfo, u64),
}

impl ClusterInfo {
    /// Without a valid keypair gossip will not function. Only useful for tests.
    pub fn new_with_invalid_keypair(contact_info: ContactInfo) -> Self {
        Self::new(contact_info, Arc::new(Keypair::new()))
    }

    pub fn new(contact_info: ContactInfo, keypair: Arc<Keypair>) -> Self {
        let mut me = Self {
            gossip: CrdsGossip::default(),
            keypair,
            gossip_leader_id: Pubkey::default(),
            entrypoint: None,
        };
        let id = contact_info.id;
        me.gossip.set_self(&id);
        me.insert_self(contact_info);
        me.push_self(&HashMap::new());
        me
    }

    pub fn insert_self(&mut self, contact_info: ContactInfo) {
        if self.id() == contact_info.id {
            let mut value = CrdsValue::ContactInfo(contact_info.clone());
            value.sign(&self.keypair);
            let _ = self.gossip.crds.insert(value, timestamp());
        }
    }

    fn push_self(&mut self, stakes: &HashMap<Pubkey, u64>) {
        let mut my_data = self.my_data();
        let now = timestamp();
        my_data.wallclock = now;
        let mut entry = CrdsValue::ContactInfo(my_data);
        entry.sign(&self.keypair);
        self.gossip.refresh_push_active_set(stakes);
        self.gossip.process_push_message(&[entry], now);
    }

    // TODO kill insert_info, only used by tests
    pub fn insert_info(&mut self, contact_info: ContactInfo) {
        let mut value = CrdsValue::ContactInfo(contact_info);
        value.sign(&self.keypair);
        let _ = self.gossip.crds.insert(value, timestamp());
    }

    pub fn set_entrypoint(&mut self, entrypoint: ContactInfo) {
        self.entrypoint = Some(entrypoint)
    }

    pub fn id(&self) -> Pubkey {
        self.gossip.id
    }

    pub fn lookup(&self, id: &Pubkey) -> Option<&ContactInfo> {
        let entry = CrdsValueLabel::ContactInfo(*id);
        self.gossip
            .crds
            .lookup(&entry)
            .and_then(|x| x.contact_info())
    }

    pub fn my_data(&self) -> ContactInfo {
        self.lookup(&self.id()).cloned().unwrap()
    }

    // Deprecated: don't use leader_data().
    pub fn leader_data(&self) -> Option<&ContactInfo> {
        let leader_id = self.gossip_leader_id;
        if leader_id == Pubkey::default() {
            return None;
        }
        self.lookup(&leader_id)
    }

    pub fn contact_info_trace(&self) -> String {
        let leader_id = self.gossip_leader_id;
        let nodes: Vec<_> = self
            .rpc_peers()
            .into_iter()
            .map(|node| {
                let mut annotation = String::new();
                if node.id == leader_id {
                    annotation.push_str(" [leader]");
                }

                format!(
                    "- gossip: {:20} | {}{}\n  \
                     tpu:    {:20} |\n  \
                     rpc:    {:20} |\n",
                    node.gossip.to_string(),
                    node.id,
                    annotation,
                    node.tpu.to_string(),
                    node.rpc.to_string()
                )
            })
            .collect();

        format!(
            " Node contact info             | Node identifier\n\
             -------------------------------+------------------\n\
             {}\
             Nodes: {}",
            nodes.join(""),
            nodes.len()
        )
    }

    /// Record the id of the current leader for use by `leader_tpu_via_blobs()`
    pub fn set_leader(&mut self, leader_id: &Pubkey) {
        warn!(
            "{}: LEADER_UPDATE TO {} from {}",
            self.gossip.id, leader_id, self.gossip_leader_id,
        );
        self.gossip_leader_id = *leader_id;
    }

    pub fn push_vote(&mut self, vote: Transaction) {
        let now = timestamp();
        let vote = Vote::new(&self.id(), vote, now);
        let mut entry = CrdsValue::Vote(vote);
        entry.sign(&self.keypair);
        self.gossip.process_push_message(&[entry], now);
    }

    /// Get votes in the crds
    /// * since - The local timestamp when the vote was updated or inserted must be greater then
    /// since. This allows the bank to query for new votes only.
    ///
    /// * return - The votes, and the max local timestamp from the new set.
    pub fn get_votes(&self, since: u64) -> (Vec<Transaction>, u64) {
        let votes: Vec<_> = self
            .gossip
            .crds
            .table
            .values()
            .filter(|x| x.local_timestamp > since)
            .filter_map(|x| {
                x.value
                    .vote()
                    .map(|v| (x.local_timestamp, v.transaction.clone()))
            })
            .collect();
        let max_ts = votes.iter().map(|x| x.0).max().unwrap_or(since);
        let txs: Vec<Transaction> = votes.into_iter().map(|x| x.1).collect();
        (txs, max_ts)
    }

    pub fn purge(&mut self, now: u64) {
        self.gossip.purge(now);
    }

    pub fn rpc_peers(&self) -> Vec<ContactInfo> {
        let me = self.my_data().id;
        self.gossip
            .crds
            .table
            .values()
            .filter_map(|x| x.value.contact_info())
            .filter(|x| x.id != me)
            .filter(|x| ContactInfo::is_valid_address(&x.rpc))
            .cloned()
            .collect()
    }

    pub fn gossip_peers(&self) -> Vec<ContactInfo> {
        let me = self.my_data().id;
        self.gossip
            .crds
            .table
            .values()
            .filter_map(|x| x.value.contact_info())
            .filter(|x| x.id != me)
            .filter(|x| ContactInfo::is_valid_address(&x.gossip))
            .cloned()
            .collect()
    }

    /// compute broadcast table
    pub fn tvu_peers(&self) -> Vec<ContactInfo> {
        let me = self.my_data().id;
        self.gossip
            .crds
            .table
            .values()
            .filter_map(|x| x.value.contact_info())
            .filter(|x| ContactInfo::is_valid_address(&x.tvu))
            .filter(|x| x.id != me)
            .cloned()
            .collect()
    }

    /// all peers that have a valid tvu
    pub fn retransmit_peers(&self) -> Vec<ContactInfo> {
        let me = self.my_data().id;
        self.gossip
            .crds
            .table
            .values()
            .filter_map(|x| x.value.contact_info())
            .filter(|x| x.id != me)
            .filter(|x| ContactInfo::is_valid_address(&x.tvu))
            .cloned()
            .collect()
    }

    /// all tvu peers with valid gossip addrs
    fn repair_peers(&self) -> Vec<ContactInfo> {
        let me = self.my_data().id;
        ClusterInfo::tvu_peers(self)
            .into_iter()
            .filter(|x| x.id != me)
            .filter(|x| ContactInfo::is_valid_address(&x.gossip))
            .collect()
    }

    fn sort_by_stake<S: std::hash::BuildHasher>(
        peers: &[ContactInfo],
        stakes: &HashMap<Pubkey, u64, S>,
    ) -> Vec<(u64, ContactInfo)> {
        let mut peers_with_stakes: Vec<_> = peers
            .iter()
            .map(|c| (*stakes.get(&c.id).unwrap_or(&0), c.clone()))
            .collect();
        peers_with_stakes.sort_unstable_by(|(l_stake, l_info), (r_stake, r_info)| {
            if r_stake == l_stake {
                r_info.id.cmp(&l_info.id)
            } else {
                r_stake.cmp(&l_stake)
            }
        });
        peers_with_stakes.dedup();
        peers_with_stakes
    }

    fn sorted_retransmit_peers<S: std::hash::BuildHasher>(
        &self,
        stakes: &HashMap<Pubkey, u64, S>,
    ) -> Vec<ContactInfo> {
        let peers = self.retransmit_peers();
        let peers_with_stakes: Vec<_> = ClusterInfo::sort_by_stake(&peers, stakes);
        peers_with_stakes
            .iter()
            .map(|(_, peer)| (*peer).clone())
            .collect()
    }

    pub fn sorted_tvu_peers(&self, stakes: &HashMap<Pubkey, u64>) -> Vec<ContactInfo> {
        let peers = self.tvu_peers();
        let peers_with_stakes: Vec<_> = ClusterInfo::sort_by_stake(&peers, stakes);
        peers_with_stakes
            .iter()
            .map(|(_, peer)| (*peer).clone())
            .collect()
    }

    /// compute broadcast table
    pub fn tpu_peers(&self) -> Vec<ContactInfo> {
        let me = self.my_data().id;
        self.gossip
            .crds
            .table
            .values()
            .filter_map(|x| x.value.contact_info())
            .filter(|x| x.id != me)
            .filter(|x| ContactInfo::is_valid_address(&x.tpu))
            .cloned()
            .collect()
    }

    /// Given a node count, neighborhood size, and an initial fanout (leader -> layer 1), it
    /// calculates how many layers are needed and at what index each layer begins.
    /// The `grow` parameter is used to determine if the network should 'fanout' or keep
    /// layer capacities constant.
    pub fn describe_data_plane(
        nodes: usize,
        fanout: usize,
        hood_size: usize,
        grow: bool,
    ) -> (usize, Vec<usize>) {
        let mut layer_indices: Vec<usize> = vec![0];
        if nodes == 0 {
            (0, vec![])
        } else if nodes <= fanout {
            // single layer data plane
            (1, layer_indices)
        } else {
            //layer 1 is going to be the first num fanout nodes, so exclude those
            let mut remaining_nodes = nodes - fanout;
            layer_indices.push(fanout);
            let mut num_layers = 2;
            let mut num_neighborhoods = fanout / 2;
            let mut layer_capacity = hood_size * num_neighborhoods;
            while remaining_nodes > 0 {
                if remaining_nodes > layer_capacity {
                    // Needs more layers.
                    num_layers += 1;
                    remaining_nodes -= layer_capacity;
                    let end = *layer_indices.last().unwrap();
                    layer_indices.push(layer_capacity + end);

                    if grow {
                        // Next layer's capacity
                        num_neighborhoods *= num_neighborhoods;
                        layer_capacity = hood_size * num_neighborhoods;
                    }
                } else {
                    //everything will now fit in the layers we have
                    let end = *layer_indices.last().unwrap();
                    layer_indices.push(layer_capacity + end);
                    break;
                }
            }
            assert_eq!(num_layers, layer_indices.len() - 1);
            (num_layers, layer_indices)
        }
    }

    fn localize_item(
        layer_indices: &[usize],
        hood_size: usize,
        select_index: usize,
        curr_index: usize,
    ) -> Option<(Locality)> {
        let end = layer_indices.len() - 1;
        let next = min(end, curr_index + 1);
        let value = layer_indices[curr_index];
        let localized = select_index >= value && select_index < layer_indices[next];
        let mut locality = Locality::default();
        if localized {
            match curr_index {
                _ if curr_index == 0 => {
                    locality.layer_ix = 0;
                    locality.layer_bounds = (0, hood_size);
                    locality.neighbor_bounds = locality.layer_bounds;
                    if next == end {
                        locality.child_layer_bounds = None;
                        locality.child_layer_peers = vec![];
                    } else {
                        locality.child_layer_bounds =
                            Some((layer_indices[next], layer_indices[next + 1]));
                        locality.child_layer_peers = ClusterInfo::lower_layer_peers(
                            select_index,
                            layer_indices[next],
                            layer_indices[next + 1],
                            hood_size,
                        );
                    }
                }
                _ if curr_index == end => {
                    locality.layer_ix = end;
                    locality.layer_bounds = (end - hood_size, end);
                    locality.neighbor_bounds = locality.layer_bounds;
                    locality.child_layer_bounds = None;
                    locality.child_layer_peers = vec![];
                }
                ix => {
                    let hood_ix = (select_index - value) / hood_size;
                    locality.layer_ix = ix;
                    locality.layer_bounds = (value, layer_indices[next]);
                    locality.neighbor_bounds = (
                        ((hood_ix * hood_size) + value),
                        ((hood_ix + 1) * hood_size + value),
                    );
                    if next == end {
                        locality.child_layer_bounds = None;
                        locality.child_layer_peers = vec![];
                    } else {
                        locality.child_layer_bounds =
                            Some((layer_indices[next], layer_indices[next + 1]));
                        locality.child_layer_peers = ClusterInfo::lower_layer_peers(
                            select_index,
                            layer_indices[next],
                            layer_indices[next + 1],
                            hood_size,
                        );
                    }
                }
            }
            Some(locality)
        } else {
            None
        }
    }

    /// Given a array of layer indices and another index, returns (as a `Locality`) the layer,
    /// layer-bounds and neighborhood-bounds in which the index resides
    fn localize(layer_indices: &[usize], hood_size: usize, select_index: usize) -> Locality {
        (0..layer_indices.len())
            .find_map(|i| ClusterInfo::localize_item(layer_indices, hood_size, select_index, i))
            .or_else(|| Some(Locality::default()))
            .unwrap()
    }

    fn lower_layer_peers(index: usize, start: usize, end: usize, hood_size: usize) -> Vec<usize> {
        (start..end)
            .step_by(hood_size)
            .map(|x| x + index % hood_size)
            .collect()
    }

    /// broadcast messages from the leader to layer 1 nodes
    /// # Remarks
    pub fn broadcast(
        id: &Pubkey,
        contains_last_tick: bool,
        broadcast_table: &[ContactInfo],
        s: &UdpSocket,
        blobs: &[SharedBlob],
    ) -> Result<()> {
        if broadcast_table.is_empty() {
            debug!("{}:not enough peers in cluster_info table", id);
            inc_new_counter_info!("cluster_info-broadcast-not_enough_peers_error", 1);
            Err(ClusterInfoError::NoPeers)?;
        }

        let orders = Self::create_broadcast_orders(contains_last_tick, blobs, broadcast_table);

        trace!("broadcast orders table {}", orders.len());

        let errs = Self::send_orders(id, s, orders);

        for e in errs {
            if let Err(e) = &e {
                trace!("{}: broadcast result {:?}", id, e);
            }
            e?;
        }

        inc_new_counter_info!("cluster_info-broadcast-max_idx", blobs.len());

        Ok(())
    }

    /// retransmit messages to a list of nodes
    /// # Remarks
    /// We need to avoid having obj locked while doing any io, such as the `send_to`
    pub fn retransmit_to(
        obj: &Arc<RwLock<Self>>,
        peers: &[ContactInfo],
        blob: &SharedBlob,
        s: &UdpSocket,
    ) -> Result<()> {
        let (me, orders): (ContactInfo, &[ContactInfo]) = {
            // copy to avoid locking during IO
            let s = obj.read().unwrap();
            (s.my_data().clone(), peers)
        };
        let rblob = blob.read().unwrap();
        trace!("retransmit orders {}", orders.len());
        let errs: Vec<_> = orders
            .par_iter()
            .map(|v| {
                debug!(
                    "{}: retransmit blob {} to {} {}",
                    me.id,
                    rblob.index(),
                    v.id,
                    v.tvu,
                );
                //TODO profile this, may need multiple sockets for par_iter
                assert!(rblob.meta.size <= BLOB_SIZE);
                s.send_to(&rblob.data[..rblob.meta.size], &v.tvu)
            })
            .collect();
        for e in errs {
            if let Err(e) = &e {
                inc_new_counter_info!("cluster_info-retransmit-send_to_error", 1, 1);
                error!("retransmit result {:?}", e);
            }
            e?;
        }
        Ok(())
    }

    /// retransmit messages from the leader to layer 1 nodes
    /// # Remarks
    /// We need to avoid having obj locked while doing any io, such as the `send_to`
    pub fn retransmit(obj: &Arc<RwLock<Self>>, blob: &SharedBlob, s: &UdpSocket) -> Result<()> {
        let peers = obj.read().unwrap().retransmit_peers();
        ClusterInfo::retransmit_to(obj, &peers, blob, s)
    }

    fn send_orders(
        id: &Pubkey,
        s: &UdpSocket,
        orders: Vec<(SharedBlob, Vec<&ContactInfo>)>,
    ) -> Vec<io::Result<usize>> {
        orders
            .into_iter()
            .flat_map(|(b, vs)| {
                let blob = b.read().unwrap();

                let ids_and_tvus = if log_enabled!(log::Level::Trace) {
                    let v_ids = vs.iter().map(|v| v.id);
                    let tvus = vs.iter().map(|v| v.tvu);
                    let ids_and_tvus = v_ids.zip(tvus).collect();

                    trace!(
                        "{}: BROADCAST idx: {} sz: {} to {:?} coding: {}",
                        id,
                        blob.index(),
                        blob.meta.size,
                        ids_and_tvus,
                        blob.is_coding()
                    );

                    ids_and_tvus
                } else {
                    vec![]
                };

                assert!(blob.meta.size <= BLOB_SIZE);
                let send_errs_for_blob: Vec<_> = vs
                    .iter()
                    .map(move |v| {
                        let e = s.send_to(&blob.data[..blob.meta.size], &v.tvu);
                        trace!(
                            "{}: done broadcast {} to {:?}",
                            id,
                            blob.meta.size,
                            ids_and_tvus
                        );
                        e
                    })
                    .collect();
                send_errs_for_blob
            })
            .collect()
    }

    pub fn create_broadcast_orders<'a, T>(
        contains_last_tick: bool,
        blobs: &[T],
        broadcast_table: &'a [ContactInfo],
    ) -> Vec<(T, Vec<&'a ContactInfo>)>
    where
        T: Clone,
    {
        // enumerate all the blobs in the window, those are the indices
        // transmit them to nodes, starting from a different node.
        if blobs.is_empty() {
            return vec![];
        }
        let mut orders = Vec::with_capacity(blobs.len());

        let x = thread_rng().gen_range(0, broadcast_table.len());
        for (i, blob) in blobs.iter().enumerate() {
            let br_idx = (x + i) % broadcast_table.len();

            trace!("broadcast order data br_idx {}", br_idx);

            orders.push((blob.clone(), vec![&broadcast_table[br_idx]]));
        }

        if contains_last_tick {
            // Broadcast the last tick to everyone on the network so it doesn't get dropped
            // (Need to maximize probability the next leader in line sees this handoff tick
            // despite packet drops)
            // If we had a tick at max_tick_height, then we know it must be the last
            // Blob in the broadcast, There cannot be an entry that got sent after the
            // last tick, guaranteed by the PohService).
            orders.push((
                blobs.last().unwrap().clone(),
                broadcast_table.iter().collect(),
            ));
        }

        orders
    }

    pub fn window_index_request_bytes(&self, slot: u64, blob_index: u64) -> Result<Vec<u8>> {
        let req = Protocol::RequestWindowIndex(self.my_data().clone(), slot, blob_index);
        let out = serialize(&req)?;
        Ok(out)
    }
