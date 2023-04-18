    fn from(other: VecDeque<T>) -> Self {
        unsafe {
            let buf = other.buf.ptr();
            let len = other.len();
            let tail = other.tail;
            let head = other.head;
            let cap = other.cap();

            // Need to move the ring to the front of the buffer, as vec will expect this.
            if other.is_contiguous() {
                ptr::copy(buf.add(tail), buf, len);
            } else {
                if (tail - head) >= cmp::min(cap - tail, head) {
                    // There is enough free space in the centre for the shortest block so we can
                    // do this in at most three copy moves.
                    if (cap - tail) > head {
                        // right hand block is the long one; move that enough for the left
                        ptr::copy(buf.add(tail),
                                  buf.add(tail - head),
                                  cap - tail);
                        // copy left in the end
                        ptr::copy(buf, buf.add(cap - head), head);
                        // shift the new thing to the start
                        ptr::copy(buf.add(tail - head), buf, len);
                    } else {
                        // left hand block is the long one, we can do it in two!
                        ptr::copy(buf, buf.add(cap - tail), head);
                        ptr::copy(buf.add(tail), buf, cap - tail);
                    }
                } else {
                    // Need to use N swaps to move the ring
                    // We can use the space at the end of the ring as a temp store

                    let mut left_edge: usize = 0;
                    let mut right_edge: usize = tail;

                    // The general problem looks like this
                    // GHIJKLM...ABCDEF - before any swaps
                    // ABCDEFM...GHIJKL - after 1 pass of swaps
                    // ABCDEFGHIJM...KL - swap until the left edge reaches the temp store
                    //                  - then restart the algorithm with a new (smaller) store
                    // Sometimes the temp store is reached when the right edge is at the end
                    // of the buffer - this means we've hit the right order with fewer swaps!
                    // E.g
                    // EF..ABCD
                    // ABCDEF.. - after four only swaps we've finished

                    while left_edge < len && right_edge != cap {
                        let mut right_offset = 0;
                        for i in left_edge..right_edge {
                            right_offset = (i - left_edge) % (cap - right_edge);
                            let src = right_edge + right_offset;
                            ptr::swap(buf.add(i), buf.add(src));
                        }
                        let n_ops = right_edge - left_edge;
                        left_edge += n_ops;
                        right_edge += right_offset + 1;

                    }
                }

            }
            let out = Vec::from_raw_parts(buf, len, cap);
            mem::forget(other);
            out
        }
    }
