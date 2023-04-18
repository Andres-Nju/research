    fn clear_relocations(&mut self, ptr: Pointer, size: Size) -> EvalResult<'tcx> {
        // Find the start and end of the given range and its outermost relocations.
        let (first, last) = {
            // Find all relocations overlapping the given range.
            let relocations = self.relocations(ptr, size)?;
            if relocations.is_empty() {
                return Ok(());
            }

            (relocations.first().unwrap().0,
             relocations.last().unwrap().0 + self.pointer_size())
        };
        let start = ptr.offset;
        let end = start + size;

        let alloc = self.get_mut(ptr.alloc_id)?;

        // Mark parts of the outermost relocations as undefined if they partially fall outside the
        // given range.
        if first < start {
            alloc.undef_mask.set_range(first, start, false);
        }
        if last > end {
            alloc.undef_mask.set_range(end, last, false);
        }

        // Forget all the relocations.
        alloc.relocations.remove_range(first .. last);

        Ok(())
    }

    fn check_relocation_edges(&self, ptr: Pointer, size: Size) -> EvalResult<'tcx> {
        let overlapping_start = self.relocations(ptr, Size::ZERO)?.len();
        let overlapping_end = self.relocations(ptr.offset(size, self)?, Size::ZERO)?.len();
        if overlapping_start + overlapping_end != 0 {
            return err!(ReadPointerAsBytes);
        }
        Ok(())
    }
