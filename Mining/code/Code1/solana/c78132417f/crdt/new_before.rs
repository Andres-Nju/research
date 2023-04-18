    pub fn new(me: ReplicatedData) -> Crdt {
        assert_eq!(me.version, 0);
        let mut g = Crdt {
            table: HashMap::new(),
            local: HashMap::new(),
            remote: HashMap::new(),
            me: me.id,
            update_index: 1,
            timeout: Duration::from_millis(100),
        };
        g.local.insert(me.id, g.update_index);
        g.table.insert(me.id, me);
        g
    }
