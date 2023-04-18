    fn alloc(&mut self, relative_range: TextRange) -> tt::TokenId {
        let id = self.toknes.len();
        self.toknes.push(relative_range);
        tt::TokenId(id as u32)
    }
