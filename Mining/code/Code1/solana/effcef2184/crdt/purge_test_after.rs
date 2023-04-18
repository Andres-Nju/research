    fn purge_test() {
        let me = ReplicatedData::new_leader(&"127.0.0.1:1234".parse().unwrap());
        let mut crdt = Crdt::new(me.clone());
        let nxt = ReplicatedData::new_leader(&"127.0.0.2:1234".parse().unwrap());
        assert_ne!(me.id, nxt.id);
        crdt.insert(&nxt);
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.gossip_addr);
        let now = crdt.alive[&nxt.id];
        let len = crdt.table.len() as u64;
        crdt.purge(now);
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.gossip_addr);

        crdt.purge(now + len * GOSSIP_SLEEP_MILLIS * 4);
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.gossip_addr);

        crdt.purge(now + len * GOSSIP_SLEEP_MILLIS * 4 + 1);
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.gossip_addr);

        let nxt2 = ReplicatedData::new_leader(&"127.0.0.2:1234".parse().unwrap());
        assert_ne!(me.id, nxt2.id);
        assert_ne!(nxt.id, nxt2.id);
        crdt.insert(&nxt2);
        while now == crdt.alive[&nxt2.id] {
            sleep(Duration::from_millis(GOSSIP_SLEEP_MILLIS));
            crdt.insert(&nxt2);
        }
        let len = crdt.table.len() as u64;
        assert!((MIN_TABLE_SIZE as u64) < len);
        crdt.purge(now + len * GOSSIP_SLEEP_MILLIS * 4);
        assert_eq!(len as usize, crdt.table.len());
        crdt.purge(now + len * GOSSIP_SLEEP_MILLIS * 4 + 1);
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.gossip_addr);
        assert_eq!(2, crdt.table.len());
    }
