    pub fn set_root(&mut self, new_root: Slot) {
        // Remove everything reachable from `self.root` but not `new_root`,
        // as those are now unrooted.
        let remove_set = self.subtree_diff(self.root, new_root);
        for slot in remove_set {
            self.fork_infos
                .remove(&slot)
                .expect("Slots reachable from old root must exist in tree");
        }
        self.fork_infos
            .get_mut(&new_root)
            .expect("new root must exist in fork_infos map")
            .parent = None;
        self.root = new_root;
    }
