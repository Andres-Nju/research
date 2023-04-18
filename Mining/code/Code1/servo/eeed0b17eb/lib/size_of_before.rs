    fn size_of(&self, ops: &mut MallocSizeOfOps) -> usize {
        let n = self.width.size_of(ops) + self.width.size_of(ops);
        assert!(n == 0);    // It would be very strange to have a non-zero value here...
        n
    }
