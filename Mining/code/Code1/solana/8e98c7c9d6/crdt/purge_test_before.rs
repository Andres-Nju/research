    fn purge_test() {
        logger::setup();
        let me = NodeInfo::new_leader(&"127.0.0.1:1234".parse().unwrap());
        let mut crdt = Crdt::new(me.clone()).expect("Crdt::new");
        let nxt = NodeInfo::new_leader(&"127.0.0.2:1234".parse().unwrap());
        assert_ne!(me.id, nxt.id);
        crdt.set_leader(me.id);
        crdt.insert(&nxt);
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.contact_info.ncp);
        let now = crdt.alive[&nxt.id];
        crdt.purge(now);
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.contact_info.ncp);

        crdt.purge(now + GOSSIP_PURGE_MILLIS);
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.contact_info.ncp);

        crdt.purge(now + GOSSIP_PURGE_MILLIS + 1);
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.contact_info.ncp);

        let nxt2 = NodeInfo::new_leader(&"127.0.0.2:1234".parse().unwrap());
        assert_ne!(me.id, nxt2.id);
        assert_ne!(nxt.id, nxt2.id);
        crdt.insert(&nxt2);
        while now == crdt.alive[&nxt2.id] {
            sleep(Duration::from_millis(GOSSIP_SLEEP_MILLIS));
            crdt.insert(&nxt2);
        }
        let len = crdt.table.len() as u64;
        assert!((MIN_TABLE_SIZE as u64) < len);
        crdt.purge(now + GOSSIP_PURGE_MILLIS);
        assert_eq!(len as usize, crdt.table.len());
        trace!("purging");
        crdt.purge(now + GOSSIP_PURGE_MILLIS + 1);
        assert_eq!(len as usize - 1, crdt.table.len());
        let rv = crdt.gossip_request().unwrap();
        assert_eq!(rv.0, nxt.contact_info.ncp);
    }
