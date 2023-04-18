    fn get(&self, key: &Q) -> Option<&Self::Key>;
    fn take(&mut self, key: &Q) -> Option<Self::Key>;
    fn replace(&mut self, key: Self::Key) -> Option<Self::Key>;
}

#[derive(Debug)]
pub struct FailedAllocationError {
    reason: &'static str,
}

impl FailedAllocationError {
    #[inline]
    pub fn new(reason: &'static str) -> Self {
        Self { reason }
    }
}
