// Copyright 2016 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use std::thread;
use std::sync::Arc;

use mio::EventLoop;
use rocksdb::DB;

use pd::{INVALID_ID, PdClient, Error as PdError};
use kvproto::raft_serverpb::StoreIdent;
use kvproto::metapb;
use util::transport::SendCh;
use raftstore::store::{self, Msg, Store, Config as StoreConfig, keys, Peekable, Transport,
                       SnapManager};
use super::Result;
use super::config::Config;
use storage::{Storage, RaftKv};
use super::transport::RaftStoreRouter;

pub fn create_raft_storage<S>(router: S, db: Arc<DB>, cfg: &Config) -> Result<Storage>
    where S: RaftStoreRouter + 'static
{
    let engine = box RaftKv::new(db, router);
    let store = try!(Storage::from_engine(engine, &cfg.storage));
    Ok(store)
}

// Node is a wrapper for raft store.
// TODO: we will rename another better name like RaftStore later.
pub struct Node<C: PdClient + 'static> {
    cluster_id: u64,
    store: metapb::Store,
    store_cfg: StoreConfig,
    store_handle: Option<thread::JoinHandle<()>>,
    ch: SendCh<Msg>,

    pd_client: Arc<C>,
}

impl<C> Node<C>
    where C: PdClient
{
    pub fn new<T>(event_loop: &mut EventLoop<Store<T, C>>,
                  cfg: &Config,
                  pd_client: Arc<C>)
                  -> Node<C>
        where T: Transport + 'static
    {
        let mut store = metapb::Store::new();
        store.set_id(INVALID_ID);
        if cfg.advertise_addr.is_empty() {
            store.set_address(cfg.addr.clone());
        } else {
            store.set_address(cfg.advertise_addr.clone())
        }

        let ch = SendCh::new(event_loop.channel());
        Node {
            cluster_id: cfg.cluster_id,
            store: store,
            store_cfg: cfg.raft_store.clone(),
            store_handle: None,
            pd_client: pd_client,
            ch: ch,
        }
    }

    pub fn start<T>(&mut self,
                    event_loop: EventLoop<Store<T, C>>,
                    engine: Arc<DB>,
                    trans: T,
                    snap_mgr: SnapManager)
                    -> Result<()>
        where T: Transport + 'static
    {
        let bootstrapped = try!(self.pd_client
            .is_cluster_bootstrapped());
        let mut store_id = try!(self.check_store(&engine));
        if store_id == INVALID_ID {
            store_id = try!(self.bootstrap_store(&engine));
        } else if !bootstrapped {
            // We have saved data before, and the cluster must be bootstrapped.
            return Err(box_err!("store {} is not empty, but cluster {} is not bootstrapped",
                                store_id,
                                self.cluster_id));
        }

        self.store.set_id(store_id);

        if !bootstrapped {
            // cluster is not bootstrapped, and we choose first store to bootstrap
            // first region.
            let region = try!(self.bootstrap_first_region(&engine, store_id));
            try!(self.bootstrap_cluster(&engine, region));
        }

        // inform pd.
        try!(self.start_store(event_loop, store_id, engine, trans, snap_mgr));
        try!(self.pd_client
            .put_store(self.store.clone()));
        Ok(())
    }

    pub fn id(&self) -> u64 {
        self.store.get_id()
    }

    pub fn get_sendch(&self) -> SendCh<Msg> {
        self.ch.clone()
    }

    // check store, return store id for the engine.
    // If the store is not bootstrapped, use INVALID_ID.
    fn check_store(&self, engine: &DB) -> Result<u64> {
        let res = try!(engine.get_msg::<StoreIdent>(&keys::store_ident_key()));
        if res.is_none() {
            return Ok(INVALID_ID);
        }

        let ident = res.unwrap();
        if ident.get_cluster_id() != self.cluster_id {
            return Err(box_err!("store ident {:?} has mismatched cluster id with {}",
                                ident,
                                self.cluster_id));
        }

        let store_id = ident.get_store_id();
        if store_id == INVALID_ID {
            return Err(box_err!("invalid store ident {:?}", ident));
        }

        Ok(store_id)
    }

    fn alloc_id(&self) -> Result<u64> {
        let id = try!(self.pd_client.alloc_id());
        Ok(id)
    }

    fn bootstrap_store(&self, engine: &DB) -> Result<u64> {
        let store_id = try!(self.alloc_id());
        debug!("alloc store id {} ", store_id);

        try!(store::bootstrap_store(engine, self.cluster_id, store_id));

        Ok(store_id)
    }

    fn bootstrap_first_region(&self, engine: &DB, store_id: u64) -> Result<metapb::Region> {
        let region_id = try!(self.alloc_id());
        info!("alloc first region id {} for cluster {}, store {}",
              region_id,
              self.cluster_id,
              store_id);
        let peer_id = try!(self.alloc_id());
        info!("alloc first peer id {} for first region {}",
              peer_id,
              region_id);

        let region = try!(store::bootstrap_region(engine, store_id, region_id, peer_id));
        Ok(region)
    }

    fn bootstrap_cluster(&mut self, engine: &DB, region: metapb::Region) -> Result<()> {
        let region_id = region.get_id();
        match self.pd_client.bootstrap_cluster(self.store.clone(), region) {
            Err(PdError::ClusterBootstrapped(_)) => {
                error!("cluster {} is already bootstrapped", self.cluster_id);
                try!(store::clear_region(engine, region_id));
                Ok(())
            }
            // TODO: should we clean region for other errors too?
            Err(e) => panic!("bootstrap cluster {} err: {:?}", self.cluster_id, e),
            Ok(_) => {
                info!("bootstrap cluster {} ok", self.cluster_id);
                Ok(())
            }
        }
    }

    fn start_store<T>(&mut self,
                      mut event_loop: EventLoop<Store<T, C>>,
                      store_id: u64,
                      db: Arc<DB>,
                      trans: T,
                      snap_mgr: SnapManager)
                      -> Result<()>
        where T: Transport + 'static
    {
        info!("start raft store {} thread", store_id);

        if self.store_handle.is_some() {
            return Err(box_err!("{} is already started", store_id));
        }

        let cfg = self.store_cfg.clone();
        let pd_client = self.pd_client.clone();
        let store = self.store.clone();
        let ch = event_loop.channel();

        let builder = thread::Builder::new().name(thd_name!(format!("raftstore-{}", store_id)));
        let h = try!(builder.spawn(move || {
            let mut store = Store::new(ch, store, cfg, db, trans, pd_client, snap_mgr).unwrap();
            if let Err(e) = store.run(&mut event_loop) {
                error!("store {} run err {:?}", store_id, e);
            };
        }));

        self.store_handle = Some(h);
        Ok(())
    }

    fn stop_store(&mut self, store_id: u64) -> Result<()> {
        info!("stop raft store {} thread", store_id);
        let h = match self.store_handle.take() {
            None => return Ok(()),
            Some(h) => h,
        };

        box_try!(self.ch.send(Msg::Quit));
        if let Err(e) = h.join() {
            return Err(box_err!("join store {} thread err {:?}", store_id, e));
        }

        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        let store_id = self.store.get_id();
        self.stop_store(store_id)
    }
}

impl<C> Drop for Node<C>
    where C: PdClient
{
    fn drop(&mut self) {
        self.stop().unwrap();
    }
}
