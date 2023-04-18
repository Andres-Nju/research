    pub fn drain<R>(&mut self, range: R) -> Drain<T>
        where R: RangeBounds<usize>
    {
        // Memory safety
        //
        // When the Drain is first created, the source deque is shortened to
        // make sure no uninitialized or moved-from elements are accessible at
        // all if the Drain's destructor never gets to run.
        //
        // Drain will ptr::read out the values to remove.
        // When finished, the remaining data will be copied back to cover the hole,
        // and the head/tail values will be restored correctly.
        //
        let len = self.len();
        let start = match range.start_bound() {
            Included(&n) => n,
            Excluded(&n) => n + 1,
            Unbounded    => 0,
        };
        let end = match range.end_bound() {
            Included(&n) => n + 1,
            Excluded(&n) => n,
            Unbounded    => len,
        };
        assert!(start <= end, "drain lower bound was too large");
        assert!(end <= len, "drain upper bound was too large");

        // The deque's elements are parted into three segments:
        // * self.tail  -> drain_tail
        // * drain_tail -> drain_head
        // * drain_head -> self.head
        //
        // T = self.tail; H = self.head; t = drain_tail; h = drain_head
        //
        // We store drain_tail as self.head, and drain_head and self.head as
        // after_tail and after_head respectively on the Drain. This also
        // truncates the effective array such that if the Drain is leaked, we
        // have forgotten about the potentially moved values after the start of
        // the drain.
        //
        //        T   t   h   H
        // [. . . o o x x o o . . .]
        //
        let drain_tail = self.wrap_add(self.tail, start);
        let drain_head = self.wrap_add(self.tail, end);
        let head = self.head;

        // "forget" about the values after the start of the drain until after
        // the drain is complete and the Drain destructor is run.
        self.head = drain_tail;

        Drain {
            deque: NonNull::from(&mut *self),
            after_tail: drain_head,
            after_head: head,
            iter: Iter {
                tail: drain_tail,
                head: drain_head,
                ring: unsafe { self.buffer_as_slice() },
            },
        }
    }
