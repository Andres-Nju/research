    fn try_push(&mut self, value: T) -> Result<(), FailedAllocationError>;
}


/////////////////////////////////////////////////////////////////
// Vec

impl<T> FallibleVec<T> for Vec<T> {
    #[inline(always)]
    fn try_push(&mut self, val: T) -> Result<(), FailedAllocationError> {
        #[cfg(feature = "known_system_malloc")]
        {
            if self.capacity() == self.len() {
                try_double_vec(self)?;
                debug_assert!(self.capacity() > self.len());
            }
        }
        self.push(val);
        Ok(())
    }
}
